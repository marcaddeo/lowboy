#[allow(dead_code)]
use crate::{post::Post, user::User};
use app::App;
use askama::Template;
use axum::{response::sse::Event, routing::get, Router};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

mod app;
mod controller;
mod id;
mod post;
mod user;

#[derive(Template)]
#[template(path = "components/post.html")]
struct PostTemplate<'p> {
    post: &'p Post,
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

    let (tx, rx) = flume::bounded::<Event>(32);

    let app = App::new(rx).await;

    let scheduler = tokio_cron_scheduler::JobScheduler::new()
        .await
        .expect("job scheduler should be created");
    scheduler.start().await.expect("scheduler should start");

    let ctx = app.clone();
    scheduler
        .add(
            tokio_cron_scheduler::Job::new_async("every 30 seconds", move |_, _| {
                let ctx = ctx.clone();
                let tx = tx.clone();
                Box::pin(async move {
                    let mut post = Post::fake();
                    let user = User::insert(&post.author, &ctx.database)
                        .await
                        .expect("inserting user should work");
                    post.author = user;
                    let post = Post::insert(post, &ctx.database)
                        .await
                        .expect("inserting post should work");

                    tx.send(
                        Event::default()
                            .event("NewPost")
                            .data(PostTemplate { post: &post }.render().unwrap()),
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

    // build our application with a route
    let app = Router::new()
        .nest_service("/dist", ServeDir::new("dist"))
        .route("/events", get(controller::events))
        .route("/", get(controller::home))
        .with_state(app);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
