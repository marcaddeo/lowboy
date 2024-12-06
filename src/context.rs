use axum::response::sse::Event;
use diesel::sqlite::SqliteConnection;
use diesel::ConnectionError;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::{AsyncConnection, SimpleAsyncConnection};
use dyn_clone::DynClone;
use flume::{Receiver, Sender};
use futures::FutureExt;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use tokio_cron_scheduler::JobScheduler;

use crate::auth::RegistrationDetails;
use crate::config::Config;
use crate::model::LowboyUserRecord;
use crate::{Connection, Events};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[error(transparent)]
    DieselConnection(#[from] diesel::ConnectionError),

    #[error(transparent)]
    PoolBuild(#[from] deadpool::managed::BuildError),

    #[error(transparent)]
    PoolConnection(
        #[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>,
    ),

    #[error(transparent)]
    JobScheduler(#[from] tokio_cron_scheduler::JobSchedulerError),

    #[error(transparent)]
    LettreSmtp(#[from] lettre::transport::smtp::Error),

    #[error(transparent)]
    LettreAddress(#[from] lettre::address::AddressError),

    #[error(transparent)]
    LettreError(#[from] lettre::error::Error),

    #[error(transparent)]
    App(#[from] anyhow::Error),
}

impl From<Error> for ConnectionError {
    fn from(value: Error) -> Self {
        match value {
            Error::DieselConnection(e) => e,
            Error::Diesel(e) => Self::CouldntSetupConfiguration(e),
            _ => unreachable!(),
        }
    }
}

pub trait Context: Send + Sync + 'static {
    fn database(&self) -> &Pool<Connection>;
    fn events(&self) -> &Events;
    fn scheduler(&self) -> &JobScheduler;
    fn mailer(&self) -> Option<&AsyncSmtpTransport<Tokio1Executor>>;
}

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait AppContext: Context + DynClone {
    fn create(
        database: Pool<Connection>,
        events: Events,
        scheduler: JobScheduler,
        mailer: Option<AsyncSmtpTransport<Tokio1Executor>>,
    ) -> Result<Self>
    where
        Self: Sized;

    async fn on_new_user(
        &self,
        record: &LowboyUserRecord,
        details: RegistrationDetails,
    ) -> Result<()> {
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
    pub mailer: Option<AsyncSmtpTransport<Tokio1Executor>>,
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

    fn mailer(&self) -> Option<&AsyncSmtpTransport<Tokio1Executor>> {
        self.mailer.as_ref()
    }
}

impl AppContext for LowboyContext {
    fn create(
        database: Pool<Connection>,
        events: Events,
        scheduler: JobScheduler,
        mailer: Option<AsyncSmtpTransport<Tokio1Executor>>,
    ) -> Result<Self> {
        Ok(Self {
            database,
            events,
            scheduler,
            mailer,
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

    fn mailer(&self) -> Option<&AsyncSmtpTransport<Tokio1Executor>> {
        unreachable!()
    }
}

impl AppContext for () {
    fn create(
        _database: Pool<Connection>,
        _events: Events,
        _scheduler: JobScheduler,
        _mailer: Option<AsyncSmtpTransport<Tokio1Executor>>,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!()
    }
}

pub async fn create_context<AC: AppContext>(config: &Config) -> Result<AC> {
    diesel::connection::set_default_instrumentation(|| {
        Some(Box::new(diesel_tracing::TracingInstrumentation::new(true)))
    })?;

    let mut manager_config = ManagerConfig::default();
    manager_config.custom_setup = Box::new(|url| {
        async {
            let mut conn = SyncConnectionWrapper::<SqliteConnection>::establish(url)
                .await
                .map_err(Error::DieselConnection)?;

            let query = "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 30000;
            ";
            conn.batch_execute(query).await.map_err(Error::Diesel)?;

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

    let mailer: Option<AsyncSmtpTransport<Tokio1Executor>> = if let Some(conf) = &config.mailer {
        Some(
            AsyncSmtpTransport::<Tokio1Executor>::relay(&conf.smtp_relay)?
                .credentials(Credentials::new(
                    conf.smtp_username.to_string(),
                    conf.smtp_password.to_string(),
                ))
                .build(),
        )
    } else {
        None
    };

    AC::create(database, events, scheduler, mailer)
}
