use async_trait::async_trait;
use diesel::prelude::*;
use diesel::{
    deserialize::QueryableByName, result::DatabaseErrorKind, sql_query, table, Selectable,
    SqliteConnection,
};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::RunQueryDsl;
use tower_sessions::{
    session::{Id, Record},
    session_store, ExpiredDeletion, SessionStore,
};

/// An error type for SQLx stores.
#[derive(thiserror::Error, Debug)]
pub enum DieselStoreError {
    /// A variant to map `sqlx` errors.
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    /// A variant to map `deadpool_diesel` pool errors.
    #[error(transparent)]
    PoolError(#[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>),

    /// A variant to map `deadpool_diesel` interact errors.
    #[error(transparent)]
    InteractError(#[from] deadpool_diesel::InteractError),

    /// A variant to map `rmp_serde` encode errors.
    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),

    /// A variant to map `rmp_serde` decode errors.
    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),
}

impl From<DieselStoreError> for session_store::Error {
    fn from(err: DieselStoreError) -> Self {
        match err {
            DieselStoreError::Diesel(inner) => session_store::Error::Backend(inner.to_string()),
            DieselStoreError::PoolError(inner) => session_store::Error::Backend(inner.to_string()),
            DieselStoreError::InteractError(inner) => {
                session_store::Error::Backend(inner.to_string())
            }
            DieselStoreError::Decode(inner) => session_store::Error::Decode(inner.to_string()),
            DieselStoreError::Encode(inner) => session_store::Error::Encode(inner.to_string()),
        }
    }
}

table! {
    tower_session (id) {
        id -> Text,
        data -> Binary,
        expiry_date -> BigInt,
    }
}

#[derive(QueryableByName, Queryable, Insertable, Selectable, PartialEq, Debug)]
#[diesel(table_name = tower_session)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct TowerSession {
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
    /// Create a new SQLite store with the provided connection pool.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use tower_sessions_sqlx_store::{sqlx::SqlitePool, SqliteStore};
    ///
    /// # tokio_test::block_on(async {
    /// let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    /// let session_store = SqliteStore::new(pool);
    /// # })
    /// ```
    pub fn new(database: Pool<SyncConnectionWrapper<SqliteConnection>>) -> Self {
        Self { database }
    }

    /// Migrate the session schema.
    pub async fn migrate(&self) -> session_store::Result<()> {
        let query = r#"
            create table if not exists tower_session
            (
                id text primary key not null,
                data blob not null,
                expiry_date integer not null
            )
            "#;

        let mut conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;
        sql_query(query.to_string())
            .execute(&mut conn)
            .await
            .map_err(DieselStoreError::Diesel)?;

        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for DieselSqliteSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        let mut conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;
        diesel::delete(tower_session::table)
            .filter(tower_session::expiry_date.lt(chrono::Utc::now().timestamp()))
            .execute(&mut conn)
            .await
            .map_err(DieselStoreError::Diesel)?;
        Ok(())
    }
}

#[async_trait]
impl SessionStore for DieselSqliteSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        async fn try_create_with_conn(
            conn: &mut SyncConnectionWrapper<SqliteConnection>,
            record: &Record,
        ) -> session_store::Result<bool> {
            let new_session = TowerSession {
                id: record.id.to_string(),
                data: rmp_serde::to_vec(&record).unwrap(),
                expiry_date: record.expiry_date.unix_timestamp(),
            };
            let res = diesel::insert_into(tower_session::table)
                .values(&new_session)
                .execute(conn)
                .await;

            match res {
                Ok(_) => Ok(true),
                Err(diesel::result::Error::DatabaseError(kind, _)) => {
                    if let DatabaseErrorKind::UniqueViolation = kind {
                        Ok(false)
                    } else {
                        Err(DieselStoreError::Diesel(res.err().unwrap()).into())
                    }
                }
                // Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(false),
                Err(e) => Err(DieselStoreError::Diesel(e).into()),
            }
        }

        let mut conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        while !try_create_with_conn(&mut conn, record).await.unwrap() {
            record.id = Id::default(); // Generate a new ID
        }

        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        async fn save_with_conn(
            conn: &mut SyncConnectionWrapper<SqliteConnection>,
            record: &Record,
        ) -> session_store::Result<()> {
            let new_session = TowerSession {
                id: record.id.to_string(),
                data: rmp_serde::to_vec(&record).unwrap(),
                expiry_date: record.expiry_date.unix_timestamp(),
            };
            diesel::insert_into(tower_session::table)
                .values(&new_session)
                .on_conflict(tower_session::id)
                .do_update()
                .set(tower_session::expiry_date.eq(new_session.expiry_date))
                .execute(conn)
                .await
                .map_err(DieselStoreError::Diesel)?;

            Ok(())
        }
        let mut conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        save_with_conn(&mut conn, record).await?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let mut conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        let session = tower_session::dsl::tower_session
            .filter(tower_session::id.eq(session_id.to_string()))
            .filter(tower_session::expiry_date.gt(chrono::Utc::now().timestamp()))
            .get_result::<TowerSession>(&mut conn)
            .await;

        if let Ok(session) = session {
            Ok(Some(
                rmp_serde::from_slice(&session.data).map_err(DieselStoreError::Decode)?,
            ))
        } else {
            return Ok(None);
        }
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let mut conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        diesel::delete(tower_session::table)
            .filter(tower_session::id.eq(session_id.to_string()))
            .execute(&mut conn)
            .await
            .map_err(DieselStoreError::Diesel)?;

        Ok(())
    }
}
