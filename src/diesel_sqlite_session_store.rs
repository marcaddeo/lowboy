use async_trait::async_trait;
use deadpool_diesel::sqlite::Pool;
use diesel::prelude::*;
use diesel::{
    deserialize::QueryableByName,
    result::DatabaseErrorKind,
    sql_query,
    sql_types::{BigInt, Binary, Text},
    table, RunQueryDsl, Selectable, SqliteConnection,
};
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
    PoolError(#[from] deadpool_diesel::PoolError),

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
    _session {
        id -> Text,
        data -> Binary,
        expiry_date -> BigInt,
    }
}

#[derive(QueryableByName, Selectable, PartialEq, Debug)]
#[diesel(table_name = _session)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct Session {
    id: String,
    data: Vec<u8>,
    expiry_date: i64,
}

#[derive(derive_more::Debug)]
pub struct DieselSqliteSessionStore {
    #[debug(skip)]
    database: Pool,
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
    pub fn new(database: Pool) -> Self {
        Self { database }
    }

    /// Migrate the session schema.
    pub async fn migrate(&self) -> sqlx::Result<()> {
        let query = r#"
            create table if not exists _session
            (
                id text primary key not null,
                data blob not null,
                expiry_date integer not null
            )
            "#;
        let _ = self
            .database
            .get()
            .await
            .unwrap()
            .interact(|conn| sql_query(query.to_string()).execute(conn))
            .await;
        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for DieselSqliteSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        let query = r#"
            delete from _session
            where datetime(expiry_date) < datetime('now')
            "#;
        self.database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?
            .interact(|conn| {
                sql_query(query.to_string())
                    .execute(conn)
                    .map_err(DieselStoreError::Diesel)
            })
            .await
            .map_err(DieselStoreError::InteractError)??;

        Ok(())
    }
}

#[async_trait]
impl SessionStore for DieselSqliteSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        fn try_create_with_conn(
            conn: &mut SqliteConnection,
            record: &Record,
        ) -> session_store::Result<bool> {
            let query = r#"
                insert or abort into _session
                (id, data, expiry_date) values (?, ?, ?)
                "#;

            let res = sql_query(query)
                .bind::<Text, _>(record.id.to_string())
                .bind::<Binary, _>(rmp_serde::to_vec(&record).unwrap())
                .bind::<BigInt, _>(record.expiry_date.unix_timestamp())
                .execute(conn);

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

        let conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        let rec = record.clone();
        let new_record = conn
            .interact(|conn| {
                let rec = rec;
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let mut rec = rec.clone();
                    while !try_create_with_conn(conn, &rec).unwrap() {
                        rec.id = Id::default(); // Generate a new ID
                    }
                    Ok(rec)
                })
            })
            .await
            .map_err(DieselStoreError::InteractError)?
            .map_err(DieselStoreError::Diesel)?;

        record.id = new_record.id;

        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        fn save_with_conn(
            conn: &mut SqliteConnection,
            record: &Record,
        ) -> session_store::Result<()> {
            let query = r#"
                insert into _session
                (id, data, expiry_date) values (?, ?, ?)
                on conflict(id) do update set
                data = excluded.data,
                expiry_date = excluded.expiry_date
                "#;
            sql_query(query)
                .bind::<Text, _>(record.id.to_string())
                .bind::<Binary, _>(rmp_serde::to_vec(&record).unwrap())
                .bind::<BigInt, _>(record.expiry_date.unix_timestamp())
                .execute(conn)
                .map_err(DieselStoreError::Diesel)?;

            Ok(())
        }
        let conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        let rec = record.clone();
        conn.interact(|conn| {
            let rec = rec;
            save_with_conn(conn, &rec)
        })
        .await
        .map_err(DieselStoreError::InteractError)?
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        let session_id = session_id.to_string();
        let session = conn
            .interact(|conn| {
                let query = r#"
                select * from _session
                where id = ? and expiry_date > ?
                LIMIT 1
                "#;
                sql_query(query)
                    .bind::<Text, _>(session_id)
                    .bind::<BigInt, _>(chrono::Utc::now().timestamp())
                    .load::<Session>(conn)
            })
            .await
            .map_err(DieselStoreError::InteractError)?
            .map_err(DieselStoreError::Diesel)?;

        if let Some(session) = session.first() {
            Ok(Some(
                rmp_serde::from_slice(&session.data).map_err(DieselStoreError::Decode)?,
            ))
        } else {
            return Ok(None);
        }
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let conn = self
            .database
            .get()
            .await
            .map_err(DieselStoreError::PoolError)?;

        let session_id = session_id.to_string();
        conn.interact(|conn| {
            let query = r#"
                    delete from _session where id = ?
                    "#;
            sql_query(query).bind::<Text, _>(session_id).execute(conn)
        })
        .await
        .map_err(DieselStoreError::InteractError)?
        .map_err(DieselStoreError::Diesel)?;

        Ok(())
    }
}

fn is_valid_table_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
