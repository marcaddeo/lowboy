use app::App;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

mod app;
mod controller;
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

    // build our application with a route
    let app = Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .route("/events", get(controller::events))
        .route("/post", post(controller::post::create))
        .route("/", get(controller::home))
        .with_state(
            App::new()
                .await
                .expect("app should initialize successfully"),
        );

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
