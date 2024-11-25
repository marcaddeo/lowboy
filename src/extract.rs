use crate::{
    app, context::CloneableAppContext, error::LowboyError, model::FromRecord as _, AppContext,
    AuthSession, Connection,
};
use anyhow::Result;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use diesel_async::pooled_connection::deadpool::{Object, Pool};

pub struct DatabaseConnection(pub Object<Connection>);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    S: Send + Sync + AppContext,
    DatabasePool: FromRef<S>,
{
    type Rejection = LowboyError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let DatabasePool(pool) = DatabasePool::from_ref(state);
        let conn = pool.get().await?;

        Ok(Self(conn))
    }
}

struct DatabasePool(Pool<Connection>);

impl<T: AppContext> FromRef<T> for DatabasePool {
    fn from_ref(input: &T) -> Self {
        Self(input.database().clone())
    }
}

pub struct JobScheduler(pub tokio_cron_scheduler::JobScheduler);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for JobScheduler
where
    S: Send + Sync + AppContext,
    JobSchedulerInstance: FromRef<S>,
{
    type Rejection = LowboyError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let JobSchedulerInstance(instance) = JobSchedulerInstance::from_ref(state);
        Ok(Self(instance))
    }
}

struct JobSchedulerInstance(tokio_cron_scheduler::JobScheduler);

impl<T: AppContext> FromRef<T> for JobSchedulerInstance {
    fn from_ref(input: &T) -> Self {
        Self(input.scheduler().clone())
    }
}

pub struct AppUser<App: app::App<AC>, AC: CloneableAppContext>(pub Option<App::User>);

#[async_trait::async_trait]
impl<S, App, AC> FromRequestParts<S> for AppUser<App, AC>
where
    S: Send + Sync + AppContext,
    App: app::App<AC>,
    AC: CloneableAppContext,
{
    type Rejection = LowboyError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let DatabaseConnection(mut conn) =
            DatabaseConnection::from_request_parts(parts, state).await?;
        let auth_session: AuthSession = axum_login::AuthSession::from_request_parts(parts, state)
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let Some(user) = auth_session.user else {
            return Ok(Self(None));
        };
        let user = App::User::from_record(&user, &mut conn).await?;

        Ok(Self(Some(user)))
    }
}

pub struct EnsureAppUser<App: app::App<AC>, AC: CloneableAppContext>(pub App::User);

#[async_trait::async_trait]
impl<S, App, AC> FromRequestParts<S> for EnsureAppUser<App, AC>
where
    S: Send + Sync + AppContext,
    App: app::App<AC>,
    AC: CloneableAppContext,
{
    type Rejection = LowboyError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AppUser(Some(user)) = AppUser::<App, AC>::from_request_parts(parts, state).await?
        else {
            return Err(LowboyError::Unauthorized)?;
        };

        Ok(Self(user))
    }
}
