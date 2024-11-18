use app::Demo;
use lowboy::Lowboy;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

mod app;
mod controller;
mod form;
mod model;
mod schema;
mod view;

// Load .env file into the environment for debug builds.
#[cfg_attr(debug_assertions, dotenvy::load)]
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,lowboy=debug,tower_http=debug",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let lowboy = Lowboy::boot().await;

    let _ = lowboy.serve::<Demo>().await;
}
