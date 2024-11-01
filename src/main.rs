#[allow(dead_code)]
use crate::{post::Post, user::User};
use askama::Template;
use axum::{extract::State, routing::get, Router};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

mod post;
mod user;

#[derive(Clone)]
struct App {
    pub database: SqlitePool,
}

impl App {
    pub async fn new() -> Self {
        let database = &format!(
            "sqlite://{}/database.sqlite3",
            std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        );
        let database = SqlitePoolOptions::new()
            .max_connections(3)
            .connect(database)
            .await
            .unwrap();

        Self { database }
    }
}

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomeTemplate {
    user: User,
    posts: Vec<Post>,
}

async fn index(app: State<App>) -> HomeTemplate {
    let user = User::current();
    let posts = fake::vec![Post; 3..8];
    HomeTemplate { user, posts }
}

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

    let app = App::new().await;

    // build our application with a route
    let app = Router::new()
        .nest_service("/dist", ServeDir::new("dist"))
        .route("/", get(index))
        .with_state(app);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
