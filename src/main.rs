use askama::Template;
use axum::{routing::get, Router};
use fake::faker::company::en::CompanyName;
use fake::faker::job::en::Title;
use fake::faker::lorem::en::Paragraph;
use fake::faker::name::en::{FirstName, LastName};
use fake::{Dummy, Fake};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

struct User {
    first_name: String,
    last_name: String,
    byline: String,
    avatar: String,
}

impl User {
    pub fn fake() -> Self {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();

        let byline = format!(
            "{} - {}",
            Title().fake::<String>(),
            CompanyName().fake::<String>()
        );

        let avatar = format!(
            "https://avatar.iran.liara.run/username?username={}+{}",
            first_name, last_name
        );

        Self {
            first_name,
            last_name,
            byline,
            avatar,
        }
    }
}

#[derive(Dummy)]
struct Post {
    #[dummy(expr = "User::fake()")]
    author: User,
    #[dummy(faker = "Paragraph(4..10)")]
    content: String,
}

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomeTemplate {
    posts: Vec<Post>,
}

async fn index() -> HomeTemplate {
    let posts = fake::vec![Post; 3..8];
    HomeTemplate { posts }
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

    // build our application with a route
    let app = Router::new()
        .nest_service("/dist", ServeDir::new("dist"))
        .route("/", get(index));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
