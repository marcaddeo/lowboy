use app::App;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

mod app;
mod controller;
mod diesel_sqlite_session_store;
mod model;
mod view;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _ = App::new().await.expect("app should boot").serve().await;
}
