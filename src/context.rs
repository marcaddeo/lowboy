use anyhow::Result;
use axum::response::sse::Event;
use diesel::sqlite::SqliteConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use flume::{Receiver, Sender};
use tokio_cron_scheduler::JobScheduler;

use crate::{Connection, Events};

pub trait Context: Send + Sync + 'static {
    fn database(&self) -> &Pool<Connection>;
    fn events(&self) -> &Events;
    fn scheduler(&self) -> &JobScheduler;
}

pub trait AppContext: Context + Clone {
    fn create(database: Pool<Connection>, events: Events, scheduler: JobScheduler) -> Result<Self>;
}

#[derive(Clone)]
pub struct LowboyContext {
    pub database: Pool<SyncConnectionWrapper<SqliteConnection>>,
    pub events: (Sender<Event>, Receiver<Event>),
    #[allow(dead_code)]
    pub scheduler: JobScheduler,
}

impl Context for LowboyContext {
    fn database(&self) -> &Pool<Connection> {
        &self.database
    }

    fn events(&self) -> &Events {
        &self.events
    }

    fn scheduler(&self) -> &JobScheduler {
        &self.scheduler
    }
}

impl AppContext for LowboyContext {
    fn create(database: Pool<Connection>, events: Events, scheduler: JobScheduler) -> Result<Self> {
        Ok(Self {
            database,
            events,
            scheduler,
        })
    }
}

pub async fn create_context<AC: AppContext>() -> Result<AC> {
    let database =
        xdg::BaseDirectories::with_prefix("lowboy/db")?.place_data_file("database.sqlite3")?;

    let config = AsyncDieselConnectionManager::<SyncConnectionWrapper<SqliteConnection>>::new(
        database.to_str().expect("database path should be valid"),
    );

    let database = Pool::builder(config).build().unwrap();

    let events = flume::bounded::<Event>(32);

    let scheduler = tokio_cron_scheduler::JobScheduler::new()
        .await
        .expect("job scheduler should be created");
    scheduler.start().await.expect("scheduler should start");

    AC::create(database, events, scheduler)
}
