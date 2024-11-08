use crate::{controller, model, view};
use anyhow::Result;
use askama::Template as _;
use async_trait::async_trait;
use axum::{
    response::sse::Event,
    routing::{get, post},
    Router,
};
use axum_login::{
    login_required,
    tower_sessions::{ExpiredDeletion as _, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder, AuthnBackend,
};
use axum_messages::MessagesManagerLayer;
use flume::{Receiver, Sender};
use oauth2::{
    basic::{BasicClient, BasicRequestTokenError},
    http::header::{AUTHORIZATION, USER_AGENT},
    reqwest::{async_http_client, AsyncHttpClientError},
    url::Url,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenResponse as _, TokenUrl,
};
use password_auth::verify_password;
use serde::Deserialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::time::Duration;
use tokio::{signal, task::AbortHandle};
use tokio_cron_scheduler::JobScheduler;
use tower_http::services::ServeDir;
use tower_sessions::cookie::{self, Key};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{info, warn};

#[derive(Clone)]
pub struct App {
    pub database: SqlitePool,
    pub events: (Sender<Event>, Receiver<Event>),
    pub scheduler: JobScheduler,
    pub oauth: BasicClient,
}

impl App {
    pub async fn new() -> Result<Self> {
        let database =
            xdg::BaseDirectories::with_prefix("lowboy/db")?.place_data_file("database.sqlite3")?;

        let database = SqlitePoolOptions::new()
            .max_connections(3)
            .connect(database.to_str().expect("database path should be valid"))
            .await
            .unwrap();

        let (tx, rx) = flume::bounded::<Event>(32);

        let scheduler = tokio_cron_scheduler::JobScheduler::new()
            .await
            .expect("job scheduler should be created");
        scheduler.start().await.expect("scheduler should start");

        let client_id = std::env::var("CLIENT_ID")
            .map(ClientId::new)
            .expect("CLIENT_ID should be provided.");
        let client_secret = std::env::var("CLIENT_SECRET")
            .map(ClientSecret::new)
            .expect("CLIENT_SECRET should be provided");

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())?;
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())?;
        let oauth = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));

        let app = Self {
            database,
            events: (tx, rx),
            scheduler,
            oauth,
        };

        // app.generate_posts().await;

        Ok(app)
    }

    pub async fn serve(self) -> Result<()> {
        let session_store = SqliteStore::new(self.database.clone());
        session_store.migrate().await?;

        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(Duration::from_secs(60)),
        );

        // @TODO
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false) // @TODO
            .with_expiry(Expiry::OnInactivity(cookie::time::Duration::days(1)))
            .with_signed(key);

        let auth_layer = AuthManagerLayerBuilder::new(self.clone(), session_layer).build();

        let router = Router::new()
            // App routes.
            .route("/events", get(controller::events))
            .route("/post", post(controller::post::create))
            .route("/", get(controller::home))
            // Previous routes require authentication.
            .route_layer(login_required!(App, login_url = "/login"))
            // Static assets.
            .nest_service("/static", ServeDir::new("static"))
            // Auth routes.
            .route("/login", get(controller::auth::form))
            .route("/login", post(controller::auth::login))
            .route("/login/oauth", get(controller::auth::oauth))
            .route("/logout", get(controller::auth::logout))
            .layer(MessagesManagerLayer)
            .layer(auth_layer)
            .with_state(self);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
        info!("listening on {}", listener.local_addr().unwrap());

        axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(Self::shutdown_signal(Some(deletion_task.abort_handle())))
            .await?;

        deletion_task.await??;

        Ok(())
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

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.oauth.authorize_url(CsrfToken::new_random).url()
    }

    #[allow(dead_code)]
    async fn generate_posts(&self) {
        let app = self.clone();
        self.scheduler
            .add(
                tokio_cron_scheduler::Job::new_async("every 30 seconds", move |_, _| {
                    let ctx = app.clone();
                    Box::pin(async move {
                        let mut post = model::Post::fake();
                        let user = model::User::insert(&post.author, &ctx.database)
                            .await
                            .expect("inserting user should work");
                        post.author = user;
                        let post = model::Post::insert(post, &ctx.database)
                            .await
                            .expect("inserting post should work");

                        let (tx, _) = ctx.events;
                        tx.send(
                            Event::default()
                                .event("NewPost")
                                .data(view::Post { post: post.clone() }.render().unwrap()),
                        )
                        .unwrap();

                        info!("Added new post by: {}", post.author.data.name);
                    })
                })
                .expect("job creation should succeed"),
            )
            .await
            .expect("scheduler should allow adding job");
    }
}

// @TODO
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Reqwest(reqwest::Error),

    #[error(transparent)]
    OAuth2(BasicRequestTokenError<AsyncHttpClientError>),

    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),
}

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub login: String,
    pub email: String,
    pub avatar_url: String,
    pub name: String,
}

#[async_trait]
impl AuthnBackend for App {
    type User = model::User;
    type Credentials = model::Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        use model::CredentialKind;

        match credentials.kind {
            CredentialKind::Password => {
                let credentials = credentials
                    .password
                    .expect("CredentialKind::Password password field should not be none");
                let user = model::User::find_by_username_with_password(
                    &credentials.username,
                    &self.database,
                )
                .await
                .ok();

                let Some(user) = user else {
                    return Ok(None);
                };

                if user.password.is_none() {
                    return Ok(None);
                }

                tokio::task::spawn_blocking(|| {
                    Ok(verify_password(
                        credentials.password,
                        user.password.as_ref().expect("checked is_none"),
                    )
                    .is_ok()
                    .then_some(user))
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

                let mut user = model::User::from(user_info);
                user.access_token = Some(token_res.access_token().secret().to_string());
                let user = model::User::insert(&user, &self.database)
                    .await
                    .map_err(|e| warn!("{}", e))
                    .expect("shrug");

                Ok(Some(user))
            }
        }
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        Ok(model::User::find_by_id(*user_id, &self.database).await.ok())
    }
}

pub type AuthSession = axum_login::AuthSession<App>;
