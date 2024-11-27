use app::Demo;
use lowboy::Lowboy;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

mod app;
mod controller;
mod form;
mod model;
mod schema;
mod view;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    Lowboy::boot().await?.serve::<Demo>().await?;

    Ok(())
}
