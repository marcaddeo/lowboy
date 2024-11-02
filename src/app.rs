use crate::model;
use axum::response::sse::Event;
use flume::Receiver;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

#[derive(Clone)]
pub struct App {
    pub database: SqlitePool,
    pub sse_event_rx: Receiver<Event>,
}

impl App {
    pub async fn new(sse_event_rx: Receiver<Event>) -> Self {
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

        Self {
            database,
            sse_event_rx,
        }
    }
}
