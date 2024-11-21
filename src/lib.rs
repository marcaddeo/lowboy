use anyhow::Result;
use axum::{http::StatusCode, middleware, response::sse::Event, routing::get, Router};
use axum_login::{
    login_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use axum_messages::MessagesManagerLayer;
use base64::prelude::*;
use context::{create_context, CloneableAppContext};
use diesel::{
    sqlite::{Sqlite, SqliteConnection},
    QueryResult,
};
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};
use diesel_sqlite_session_store::DieselSqliteSessionStore;
use flume::{Receiver, Sender};
use std::{io::LineWriter, time::Duration};
use tokio::{signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions::cookie::{self, Key};
use tracing::{info, warn};

mod app;
pub mod auth;
mod context;
pub mod controller;
mod diesel_sqlite_session_store;
pub mod extractor;
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

#[derive(Clone)]
pub struct Lowboy<AC: AppContext> {
    context: AC,
}

struct MigrationWriter;
impl std::io::Write for MigrationWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut line = String::from_utf8(buf.into()).unwrap();

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
    pub async fn boot() -> Self {
        let context = create_context::<AC>().await.unwrap();

        let mut conn = context.database().get().await.unwrap();
        conn.spawn_blocking(|conn| Self::run_migrations(conn))
            .await
            .unwrap();

        Self { context }
    }

    fn run_migrations(conn: &mut impl MigrationHarness<Sqlite>) -> QueryResult<()> {
        HarnessWithOutput::new(conn, LineWriter::new(MigrationWriter))
            .run_pending_migrations(MIGRATIONS)
            .unwrap();
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
            .with_same_site(cookie::SameSite::Lax)
            .with_expiry(Expiry::OnInactivity(cookie::time::Duration::days(1)))
            .with_signed(session_key);

        let lowboy_auth = LowboyAuth::new(Box::new(self.context.clone()))?;
        let auth_layer = AuthManagerLayerBuilder::new(lowboy_auth, session_layer).build();

        let router = Router::new()
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

pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
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
