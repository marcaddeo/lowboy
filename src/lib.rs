use axum::{middleware, response::sse::Event, routing::get, Router};
use axum_login::{
    login_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use axum_messages::MessagesManagerLayer;
use base64::prelude::*;
use config::Config;
use context::{create_context, CloneableAppContext};
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};
use diesel_sqlite_session_store::DieselSqliteSessionStore;
use error::LowboyError;
use flume::{Receiver, Sender};
use std::{io::LineWriter, time::Duration};
use tokio::{signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions::cookie::{self, Key};
use tracing::info;

mod app;
pub mod auth;
mod config;
pub mod context;
pub mod controller;
mod diesel_sqlite_session_store;
pub mod error;
pub mod extract;
mod mailer;
pub mod model;
mod schema;
pub mod view;

pub use {
    app::App,
    auth::{AuthSession, LowboyAuth},
    context::{AppContext, Context, LowboyContext},
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub type Connection = SyncConnectionWrapper<SqliteConnection>;
pub type Events = (Sender<Event>, Receiver<Event>);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] crate::config::Error),

    #[error(transparent)]
    Context(#[from] crate::context::Error),

    #[error(transparent)]
    Auth(#[from] crate::auth::Error),

    #[error(transparent)]
    Pool(#[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>),

    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[error(transparent)]
    SessionStore(#[from] tower_sessions::session_store::Error),

    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Notify(#[from] notify::Error),

    #[error(transparent)]
    Migration(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Clone)]
pub struct Lowboy<AC: AppContext> {
    config: Config,
    context: AC,
}

struct MigrationWriter;
impl std::io::Write for MigrationWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut line = String::from_utf8(buf.into()).map_err(std::io::Error::other)?;

        // Remove trailing newline.
        line.pop();

        info!("{}", line);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<AC: CloneableAppContext> Lowboy<AC> {
    pub async fn boot() -> Result<Self> {
        let config = Config::load(None)?;
        let context = create_context::<AC>(&config).await?;

        let mut conn = context.database().get().await?;
        conn.spawn_blocking(|conn| Ok(Self::run_migrations(conn)))
            .await??;

        Ok(Self { config, context })
    }

    fn run_migrations(conn: &mut impl MigrationHarness<Sqlite>) -> Result<()> {
        HarnessWithOutput::new(conn, LineWriter::new(MigrationWriter))
            .run_pending_migrations(MIGRATIONS)?;
        Ok(())
    }

    pub async fn serve<App: app::App<AC>>(self) -> Result<()> {
        let session_store = DieselSqliteSessionStore::new(self.context.database().clone());
        session_store.migrate().await?;

        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(Duration::from_secs(60)),
        );
        let session_key = BASE64_STANDARD.decode(self.config.session_key)?;
        let session_key = Key::from(session_key.as_slice());

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false) // @TODO
            .with_expiry(Expiry::OnInactivity(cookie::time::Duration::days(1)))
            .with_signed(session_key);

        let lowboy_auth =
            LowboyAuth::new(Box::new(self.context.clone()), self.config.oauth_providers)?;
        let auth_layer = AuthManagerLayerBuilder::new(lowboy_auth, session_layer).build();

        let router = Router::new()
            .fallback(|| async { LowboyError::NotFound })
            // App routes.
            .route("/events", get(controller::events::<AC>))
            // Previous routes require authentication.
            .route_layer(login_required!(LowboyAuth, login_url = "/login"))
            // Static assets.
            .nest_service("/static", ServeDir::new("static"))
            .merge(App::routes())
            .merge(App::auth_routes::<App>())
            .layer(middleware::map_response_with_state(
                self.context.clone(),
                view::render_view::<App, AC>,
            ))
            .layer(middleware::map_response_with_state(
                self.context.clone(),
                view::error_page::<App, AC>,
            ))
            .layer(MessagesManagerLayer)
            .layer(auth_layer)
            .layer(middleware::map_response_with_state(
                self.context.clone(),
                view::error_page::<App, AC>,
            ));

        // Enable livereload for debug builds.
        #[cfg(debug_assertions)]
        let (router, _watcher) = livereload(router)?;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
        info!("listening on {}", listener.local_addr()?);

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

#[cfg(debug_assertions)]
fn not_htmx_predicate(req: &axum::extract::Request) -> bool {
    !req.headers().contains_key("hx-request")
}

#[cfg(debug_assertions)]
fn livereload<AC: CloneableAppContext>(
    router: axum::Router<AC>,
) -> Result<(axum::Router<AC>, notify::FsEventWatcher)> {
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
