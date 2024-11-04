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
use password_auth::verify_password;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::time::Duration;
use tokio::{signal, task::AbortHandle};
use tokio_cron_scheduler::JobScheduler;
use tower_http::services::ServeDir;
use tower_sessions::cookie::{self, Key};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::info;

#[derive(Clone)]
pub struct App {
    pub database: SqlitePool,
    pub events: (Sender<Event>, Receiver<Event>),
    pub scheduler: JobScheduler,
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

        let app = Self {
            database,
            events: (tx, rx),
            scheduler,
        };

        app.generate_posts().await;

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
            // Sun guy.
            .route("/sun-guy", get(controller::sun_guy))
            // Static assets.
            .nest_service("/static", ServeDir::new("static"))
            // Auth routes.
            .route("/login", get(controller::auth::form))
            .route("/login", post(controller::auth::login))
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

                        info!(
                            "Added new post by: {} {}",
                            post.author.first_name, post.author.last_name
                        );
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
    TaskJoin(#[from] tokio::task::JoinError),
}

#[async_trait]
impl AuthnBackend for App {
    type User = model::User;
    type Credentials = model::Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = model::User::find_by_username(&creds.username, &self.database)
            .await
            .ok();

        let Some(user) = user else {
            return Ok(None);
        };

        tokio::task::spawn_blocking(|| {
            Ok(verify_password(creds.password, &user.password)
                .is_ok()
                .then_some(user))
        })
        .await?
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        Ok(model::User::find_by_id(*user_id, &self.database).await.ok())
    }
}

pub type AuthSession = axum_login::AuthSession<App>;
