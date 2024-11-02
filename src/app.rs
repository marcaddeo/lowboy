use crate::{model, view};
use askama::Template as _;
use axum::response::sse::Event;
use flume::{Receiver, Sender};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tokio_cron_scheduler::JobScheduler;
use tracing::info;

#[derive(Clone)]
pub struct App {
    pub database: SqlitePool,
    pub events: (Sender<Event>, Receiver<Event>),
    pub scheduler: JobScheduler,
}

impl App {
    pub async fn new() -> Self {
        let database = &format!(
            "sqlite://{}/target/database.sqlite3",
            std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        );
        let database = SqlitePoolOptions::new()
            .max_connections(3)
            .connect(database)
            .await
            .unwrap();

        let schemas: Vec<(&str, &str)> = vec![model::User::SCHEMA, model::Post::SCHEMA];
        for (table, schema) in &schemas {
            let query = format!("CREATE TABLE IF NOT EXISTS {} ({})", table, schema);
            sqlx::query(&query).execute(&database).await.unwrap();
        }

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

        app.generate_posts();

        app
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
                                .data(view::Post { post: &post }.render().unwrap()),
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
