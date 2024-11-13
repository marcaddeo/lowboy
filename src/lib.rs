use anyhow::Result;
use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    middleware,
    response::sse::Event,
    routing::{get, post},
    Router,
};
use axum_login::{
    login_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder, AuthnBackend,
};
use axum_messages::MessagesManagerLayer;
use base64::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_sqlite_session_store::DieselSqliteSessionStore;
use flume::{Receiver, Sender};
use model::{NewUserRecord, User, UserRecord};
use oauth2::{
    basic::{BasicClient, BasicRequestTokenError},
    http::header::{AUTHORIZATION, USER_AGENT},
    reqwest::{async_http_client, AsyncHttpClientError},
    url::Url,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenResponse, TokenUrl,
};
use password_auth::verify_password;
use serde::Deserialize;
use std::time::Duration;
use tokio::{signal, task::AbortHandle};
use tokio_cron_scheduler::JobScheduler;
use tower_http::services::ServeDir;
use tower_sessions::cookie::{self, Key};
use tracing::{info, warn};

mod controller;
mod diesel_sqlite_session_store;
pub mod model;
mod schema;
pub mod view;

pub type Connection = SyncConnectionWrapper<SqliteConnection>;
pub type Events = (Sender<Event>, Receiver<Event>);

pub trait Context: Send + Sync + 'static {
    fn database(&self) -> &Pool<Connection>;
    fn events(&self) -> &Events;
    fn scheduler(&self) -> &JobScheduler;
}

pub trait AppContext: Context + Clone {
    fn create(database: Pool<Connection>, events: Events, scheduler: JobScheduler) -> Result<Self>;
}

#[derive(Clone)]
pub struct LowboyContext {
    pub database: Pool<SyncConnectionWrapper<SqliteConnection>>,
    pub events: (Sender<Event>, Receiver<Event>),
    #[allow(dead_code)]
    pub scheduler: JobScheduler,
}

impl Context for LowboyContext {
    fn database(&self) -> &Pool<Connection> {
        &self.database
    }

    fn events(&self) -> &Events {
        &self.events
    }

    fn scheduler(&self) -> &JobScheduler {
        &self.scheduler
    }
}

impl AppContext for LowboyContext {
    fn create(database: Pool<Connection>, events: Events, scheduler: JobScheduler) -> Result<Self> {
        Ok(Self {
            database,
            events,
            scheduler,
        })
    }
}

pub async fn create_context<AC: AppContext>() -> Result<AC> {
    let database =
        xdg::BaseDirectories::with_prefix("lowboy/db")?.place_data_file("database.sqlite3")?;

    let config = AsyncDieselConnectionManager::<SyncConnectionWrapper<SqliteConnection>>::new(
        database.to_str().expect("database path should be valid"),
    );

    let database = Pool::builder(config).build().unwrap();

    let events = flume::bounded::<Event>(32);

    let scheduler = tokio_cron_scheduler::JobScheduler::new()
        .await
        .expect("job scheduler should be created");
    scheduler.start().await.expect("scheduler should start");

    AC::create(database, events, scheduler)
}

pub trait App<AC: AppContext>: Send {
    fn name() -> &'static str;

    fn routes() -> Router<AC>;
}

#[derive(Clone)]
pub struct LowboyAuth {
    oauth: BasicClient,
    database: Pool<Connection>,
}

impl LowboyAuth {
    pub fn new(database: Pool<Connection>) -> Result<Self> {
        let client_id = std::env::var("CLIENT_ID")
            .map(ClientId::new)
            .expect("CLIENT_ID should be provided.");
        let client_secret = std::env::var("CLIENT_SECRET")
            .map(ClientSecret::new)
            .expect("CLIENT_SECRET should be provided");

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())?;
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())?;
        let oauth = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));

        Ok(Self { oauth, database })
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.oauth.authorize_url(CsrfToken::new_random).url()
    }
}

#[derive(Clone)]
pub struct Lowboy<T: AppContext> {
    context: T,
}

impl<T: AppContext> Lowboy<T> {
    pub async fn boot() -> Self {
        let context = create_context().await.unwrap();

        Self { context }
    }

    pub async fn serve<App: self::App<T>>(self) -> Result<()> {
        let session_store = DieselSqliteSessionStore::new(self.context.database().clone());
        session_store.migrate().await?;

        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(Duration::from_secs(60)),
        );
        let session_key = std::env::var("SESSION_KEY").ok();
        let session_key = if let Some(session_key) = session_key {
            let session_key = BASE64_STANDARD.decode(session_key)?;
            Key::from(session_key.as_slice())
        } else {
            warn!("Could not get SESSION_KEY from environment. Falling back to generated key. This will invalidate any sessions when the server is stopped.");
            Key::generate()
        };

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false) // @TODO
            .with_expiry(Expiry::OnInactivity(cookie::time::Duration::days(1)))
            .with_signed(session_key);

        let lowboy_auth = LowboyAuth::new(self.context.database().clone())?;
        let auth_layer = AuthManagerLayerBuilder::new(lowboy_auth, session_layer).build();

        let app_routes = App::routes();

        let router = Router::new()
            // App routes.
            .route("/events", get(controller::events::<T>))
            // Previous routes require authentication.
            .route_layer(login_required!(LowboyAuth, login_url = "/login"))
            // Static assets.
            .nest_service("/static", ServeDir::new("static"))
            // Auth routes.
            .route("/register", get(controller::auth::register_form))
            .route("/register", post(controller::auth::register::<T>))
            .route("/login", get(controller::auth::form))
            .route("/login", post(controller::auth::login))
            .route("/login/oauth", get(controller::auth::oauth))
            .route("/logout", get(controller::auth::logout))
            .merge(app_routes)
            .layer(middleware::map_response_with_state(
                self.context.clone(),
                view::render_view::<T>,
            ))
            .layer(MessagesManagerLayer)
            .layer(auth_layer);

        // Enable livereload for debug builds.
        #[cfg(debug_assertions)]
        let (router, _watcher) = livereload(router)?;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
        info!("listening on {}", listener.local_addr().unwrap());

        axum::serve(
            listener,
            router.with_state(self.context).into_make_service(),
        )
        .with_graceful_shutdown(shutdown_signal(Some(deletion_task.abort_handle())))
        .await?;

        deletion_task.await??;

        Ok(())
    }
}

pub async fn shutdown_signal(abort_handle: Option<AbortHandle>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => { if let Some(abort_handle) = abort_handle { abort_handle.abort() } },
        _ = terminate => { if let Some(abort_handle) = abort_handle { abort_handle.abort() } },
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(reqwest::Error),

    #[error(transparent)]
    OAuth2(BasicRequestTokenError<AsyncHttpClientError>),

    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Deadpool(#[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>),

    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
}

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub login: String,
    pub email: String,
    pub avatar_url: String,
    pub name: String,
}

#[async_trait]
impl AuthnBackend for LowboyAuth {
    type User = UserRecord;
    type Credentials = model::Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        use model::CredentialKind;

        let mut conn = self.database.get().await?;

        match credentials.kind {
            CredentialKind::Password => {
                let credentials = credentials
                    .password
                    .expect("CredentialKind::Password password field should not be none");
                let user = User::find_by_username_having_password(&credentials.username, &mut conn)
                    .await?;

                tokio::task::spawn_blocking(|| {
                    Ok(verify_password(
                        credentials.password,
                        user.password.as_ref().expect("checked is_none"),
                    )
                    .is_ok()
                    .then_some(user.into()))
                })
                .await?
            }
            CredentialKind::OAuth => {
                let credentials = credentials
                    .oauth
                    .expect("CredentialKind::OAuth oauth field should not be none");
                // Ensure the CSRF state has not been tampered with.
                if credentials.old_state.secret() != credentials.new_state.secret() {
                    return Ok(None);
                };

                // Process authorization code, expecting a token response back.
                let token_res = self
                    .oauth
                    .exchange_code(AuthorizationCode::new(credentials.code))
                    .request_async(async_http_client)
                    .await
                    .map_err(Self::Error::OAuth2)?;

                // Use access token to request user info.
                let user_info = reqwest::Client::new()
                    .get("https://api.github.com/user")
                    .header(USER_AGENT.as_str(), "lowboy")
                    .header(
                        AUTHORIZATION.as_str(),
                        format!("Bearer {}", token_res.access_token().secret()),
                    )
                    .send()
                    .await
                    .map_err(Self::Error::Reqwest)?
                    .json::<GitHubUserInfo>()
                    .await
                    .map_err(Self::Error::Reqwest)?;

                // Persist user in our database so we can use `get_user`.
                let access_token = token_res.access_token().secret();
                let new_user = NewUserRecord {
                    username: &user_info.login,
                    email: &user_info.email,
                    password: None,
                    access_token: Some(access_token),
                };
                let record = new_user
                    .create_or_update(
                        &user_info.name,
                        None,
                        Some(&user_info.avatar_url),
                        &mut conn,
                    )
                    .await?;

                Ok(Some(record))
            }
        }
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        let mut conn = self.database.get().await?;
        Ok(Some(User::find(*user_id, &mut conn).await?.into()))
    }
}

pub type AuthSession = axum_login::AuthSession<LowboyAuth>;

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[cfg(debug_assertions)]
fn not_htmx_predicate<T>(req: &axum::extract::Request<T>) -> bool {
    !req.headers().contains_key("hx-request")
}

#[cfg(debug_assertions)]
fn livereload<T: AppContext>(
    router: axum::Router<T>,
) -> Result<(axum::Router<T>, notify::FsEventWatcher)> {
    use notify::Watcher;

    let livereload = tower_livereload::LiveReloadLayer::new();
    let reloader = livereload.reloader();

    let router = router.layer(livereload.request_predicate(not_htmx_predicate));

    let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
    watcher.watch(
        std::path::Path::new("static"),
        notify::RecursiveMode::Recursive,
    )?;

    Ok((router, watcher))
}
