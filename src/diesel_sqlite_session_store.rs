use ::tower_sessions::{
    session::{Id, Record},
    session_store, ExpiredDeletion, SessionStore,
};
use async_trait::async_trait;
use diesel::prelude::*;
use diesel::{
    deserialize::QueryableByName, result::DatabaseErrorKind, sql_query, table, Selectable,
    SqliteConnection,
};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::RunQueryDsl;

type Result<T> = std::result::Result<T, Error>;

/// An error type for SQLx stores.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A variant to map `sqlx` errors.
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    /// A variant to map `deadpool_diesel` pool errors.
    #[error(transparent)]
    Pool(#[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>),

    /// A variant to map `deadpool_diesel` interact errors.
    #[error(transparent)]
    Interact(#[from] deadpool_diesel::InteractError),

    /// A variant to map `rmp_serde` encode errors.
    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),

    /// A variant to map `rmp_serde` decode errors.
    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),
}

impl From<Error> for session_store::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Diesel(inner) => session_store::Error::Backend(inner.to_string()),
            Error::Pool(inner) => session_store::Error::Backend(inner.to_string()),
            Error::Interact(inner) => session_store::Error::Backend(inner.to_string()),
            Error::Decode(inner) => session_store::Error::Decode(inner.to_string()),
            Error::Encode(inner) => session_store::Error::Encode(inner.to_string()),
        }
    }
}

table! {
    tower_sessions (id) {
        id -> Text,
        data -> Binary,
        expiry_date -> BigInt,
    }
}

#[derive(QueryableByName, Queryable, Insertable, Selectable, PartialEq, Debug)]
#[diesel(table_name = tower_sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TowerSession {
    id: String,
    data: Vec<u8>,
    expiry_date: i64,
}

#[derive(derive_more::Debug, Clone)]
pub struct DieselSqliteSessionStore {
    #[debug(skip)]
    database: Pool<SyncConnectionWrapper<SqliteConnection>>,
}

impl DieselSqliteSessionStore {
    pub fn new(database: Pool<SyncConnectionWrapper<SqliteConnection>>) -> Self {
        Self { database }
    }

    /// Migrate the session schema.
    pub async fn migrate(&self) -> session_store::Result<()> {
        let query = r#"
            create table if not exists tower_sessions
            (
                id text primary key not null,
                data blob not null,
                expiry_date integer not null
            )
            "#;

        let mut conn = self.database.get().await.map_err(Error::Pool)?;
        sql_query(query.to_string())
            .execute(&mut conn)
            .await
            .map_err(Error::Diesel)?;

        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for DieselSqliteSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        let mut conn = self.database.get().await.map_err(Error::Pool)?;
        diesel::delete(tower_sessions::table)
            .filter(tower_sessions::expiry_date.lt(chrono::Utc::now().timestamp()))
            .execute(&mut conn)
            .await
            .map_err(Error::Diesel)?;
        Ok(())
    }
}

#[async_trait]
impl SessionStore for DieselSqliteSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        async fn try_create_with_conn(
            conn: &mut SyncConnectionWrapper<SqliteConnection>,
            record: &Record,
        ) -> Result<bool> {
            let new_session = TowerSession {
                id: record.id.to_string(),
                data: rmp_serde::to_vec(&record)?,
                expiry_date: record.expiry_date.unix_timestamp(),
            };
            let res = diesel::insert_into(tower_sessions::table)
                .values(&new_session)
                .execute(conn)
                .await;

            match res {
                Ok(_) => Ok(true),
                Err(err @ diesel::result::Error::DatabaseError(kind, _)) => {
                    if let DatabaseErrorKind::UniqueViolation = kind {
                        Ok(false)
                    } else {
                        Err(Error::Diesel(err))
                    }
                }
                Err(e) => Err(Error::Diesel(e)),
            }
        }

        let mut conn = self.database.get().await.map_err(Error::Pool)?;

        while !try_create_with_conn(&mut conn, record).await? {
            record.id = Id::default(); // Generate a new ID
        }

        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        async fn save_with_conn(
            conn: &mut SyncConnectionWrapper<SqliteConnection>,
            record: &Record,
        ) -> Result<()> {
            let new_session = TowerSession {
                id: record.id.to_string(),
                data: rmp_serde::to_vec(&record)?,
                expiry_date: record.expiry_date.unix_timestamp(),
            };
            diesel::insert_into(tower_sessions::table)
                .values(&new_session)
                .on_conflict(tower_sessions::id)
                .do_update()
                .set((
                    tower_sessions::expiry_date.eq(new_session.expiry_date),
                    tower_sessions::data.eq(new_session.data.clone()),
                ))
                .execute(conn)
                .await
                .map_err(Error::Diesel)?;

            Ok(())
        }
        let mut conn = self.database.get().await.map_err(Error::Pool)?;

        save_with_conn(&mut conn, record).await?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let mut conn = self.database.get().await.map_err(Error::Pool)?;

        let session = tower_sessions::dsl::tower_sessions
            .filter(tower_sessions::id.eq(session_id.to_string()))
            .filter(tower_sessions::expiry_date.gt(chrono::Utc::now().timestamp()))
            .get_result::<TowerSession>(&mut conn)
            .await;

        if let Ok(session) = session {
            Ok(Some(
                rmp_serde::from_slice(&session.data).map_err(Error::Decode)?,
            ))
        } else {
            return Ok(None);
        }
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let mut conn = self.database.get().await.map_err(Error::Pool)?;

        diesel::delete(tower_sessions::table)
            .filter(tower_sessions::id.eq(session_id.to_string()))
            .execute(&mut conn)
            .await
            .map_err(Error::Diesel)?;

        Ok(())
    }
}
