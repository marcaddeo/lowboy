use crate::config::Config;
use crate::{auth::RegistrationDetails, model::LowboyUserRecord, Connection, Events};
use axum::response::sse::Event;
use diesel::sqlite::SqliteConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::{deadpool::Pool, ManagerConfig};
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::{AsyncConnection, SimpleAsyncConnection};
use dyn_clone::DynClone;
use flume::{Receiver, Sender};
use futures::FutureExt;
use tokio_cron_scheduler::JobScheduler;

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[error(transparent)]
    PoolBuild(#[from] deadpool::managed::BuildError),

    #[error(transparent)]
    PoolConnection(
        #[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>,
    ),

    #[error(transparent)]
    JobScheduler(#[from] tokio_cron_scheduler::JobSchedulerError),

    #[error(transparent)]
    App(#[from] anyhow::Error),
}

pub trait Context: Send + Sync + 'static {
    fn database(&self) -> &Pool<Connection>;
    fn events(&self) -> &Events;
    fn scheduler(&self) -> &JobScheduler;
}

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait AppContext: Context + DynClone {
    fn create(
        database: Pool<Connection>,
        events: Events,
        scheduler: JobScheduler,
    ) -> Result<Self, ContextError>
    where
        Self: Sized;

    async fn on_new_user(
        &self,
        record: &LowboyUserRecord,
        details: RegistrationDetails,
    ) -> Result<(), ContextError> {
        Ok(())
    }
}
dyn_clone::clone_trait_object!(AppContext);

pub trait CloneableAppContext: AppContext + Clone {}
impl<T: AppContext + Clone> CloneableAppContext for T {}

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
    fn create(
        database: Pool<Connection>,
        events: Events,
        scheduler: JobScheduler,
    ) -> Result<Self, ContextError> {
        Ok(Self {
            database,
            events,
            scheduler,
        })
    }
}

// These implementations were necessary to make extractors work. I'm pretty sure these are actually
// unreachable, hopefully 😅
impl Context for () {
    fn database(&self) -> &Pool<Connection> {
        unreachable!()
    }

    fn events(&self) -> &Events {
        unreachable!()
    }

    fn scheduler(&self) -> &JobScheduler {
        unreachable!()
    }
}

impl AppContext for () {
    fn create(
        _database: Pool<Connection>,
        _events: Events,
        _scheduler: JobScheduler,
    ) -> Result<Self, ContextError>
    where
        Self: Sized,
    {
        unreachable!()
    }
}

pub async fn create_context<AC: AppContext>(config: &Config) -> Result<AC, ContextError> {
    diesel::connection::set_default_instrumentation(|| {
        Some(Box::new(diesel_tracing::TracingInstrumentation::new(true)))
    })?;

    let mut manager_config = ManagerConfig::default();
    manager_config.custom_setup = Box::new(|url| {
        async {
            let mut conn = SyncConnectionWrapper::<SqliteConnection>::establish(url).await?;

            let query = "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 30000;
            ";
            conn.batch_execute(query).await.unwrap(); // @TODO

            Ok(conn)
        }
        .boxed()
    });

    let manager =
        AsyncDieselConnectionManager::<SyncConnectionWrapper<SqliteConnection>>::new_with_config(
            config.database_url.clone(),
            manager_config,
        );

    let database = Pool::builder(manager)
        .max_size(config.database_pool_size)
        .build()?;

    let events = flume::bounded::<Event>(32);

    let scheduler = JobScheduler::new().await?;
    scheduler.start().await?;

    AC::create(database, events, scheduler)
}
