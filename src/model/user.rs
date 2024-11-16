use crate::schema::user;
use crate::Connection;
use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::upsert::excluded;
use diesel::{insert_into, prelude::*};
use diesel::{QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use lowboy_record::prelude::*;

pub trait ModelRecord {}

// @TODO pub trait FromRecord<T: ModelRecord> {
pub trait FromRecord<T> {
    fn from_record(
        record: &T,
        conn: &mut Connection,
    ) -> impl std::future::Future<Output = QueryResult<Self>> + Send
    where
        Self: Sized;
}

pub trait LowboyUser<T>: FromRecord<T> {
    fn id(&self) -> i32;
    fn username(&self) -> &String;
    fn email(&self) -> &String;
    fn password(&self) -> &Option<String>;
    fn access_token(&self) -> &Option<String>;
}

// @TODO we need to mask the password and access token again, which requires fixing the macro to
// allow doing so.
#[apply(lowboy_record!)]
#[derive(Clone, DebugMasked, Default, Queryable, Selectable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub access_token: Option<String>,
}

impl User {
    pub async fn find(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        let record: UserRecord = user::table.find(id).first(conn).await?;
        Self::from_record(&record, conn).await
    }

    pub async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Self> {
        let record: UserRecord = user::table
            .filter(user::username.eq(username))
            .first(conn)
            .await?;
        Self::from_record(&record, conn).await
    }

    pub async fn find_by_username_having_password(
        username: &str,
        conn: &mut Connection,
    ) -> QueryResult<Self> {
        let record: UserRecord = user::table
            .filter(user::username.eq(username))
            .filter(user::password.is_not_null())
            .first(conn)
            .await?;
        Self::from_record(&record, conn).await
    }
}

impl FromRecord<UserRecord> for User {
    async fn from_record(record: &UserRecord, conn: &mut Connection) -> QueryResult<Self> {
        Self::from_record(record, conn).await
    }
}

impl LowboyUser<UserRecord> for User {
    fn id(&self) -> i32 {
        self.id
    }

    fn username(&self) -> &String {
        &self.username
    }

    fn email(&self) -> &String {
        &self.email
    }

    fn password(&self) -> &Option<String> {
        &self.password
    }

    fn access_token(&self) -> &Option<String> {
        &self.access_token
    }
}

impl<'a> NewUserRecord<'a> {
    pub async fn create_or_update(&self, conn: &mut Connection) -> QueryResult<UserRecord> {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                let user: UserRecord = insert_into(user::table)
                    .values(self)
                    .on_conflict(user::email)
                    .do_update()
                    .set(user::access_token.eq(excluded(user::access_token)))
                    .get_result(conn)
                    .await?;

                Ok(user)
            }
            .scope_boxed()
        })
        .await
    }
}

impl AuthUser for UserRecord {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        if let Some(access_token) = &self.access_token {
            return access_token.as_bytes();
        }

        if let Some(password) = &self.password {
            return password.as_bytes();
        }

        &[]
    }
}
