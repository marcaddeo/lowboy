use crate::schema::lowboy_user;
use crate::Connection;
use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::upsert::excluded;
use diesel::{insert_into, prelude::*};
use diesel::{QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use gravatar_api::avatars as gravatars;
use lowboy_record::prelude::*;

pub trait ModelRecord {}

// @TODO pub trait FromRecord<T: ModelRecord> {
#[async_trait::async_trait]
pub trait FromRecord<T> {
    async fn from_record(record: &T, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized;
}

pub trait LowboyUserTrait<T>: FromRecord<T> {
    fn id(&self) -> i32;
    fn username(&self) -> &String;
    fn email(&self) -> &String;
    fn password(&self) -> &Option<String>;
    fn access_token(&self) -> &Option<String>;
    fn gravatar(&self) -> String {
        gravatars::Avatar::builder(self.email())
            .size(256)
            .default(gravatars::Default::MysteryPerson)
            .rating(gravatars::Rating::Pg)
            .build()
            .image_url()
            .to_string()
    }
}

// @TODO we need to mask the password and access token again, which requires fixing the macro to
// allow doing so.
#[apply(lowboy_record!)]
#[derive(Clone, DebugMasked, Default, Queryable, Selectable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::schema::lowboy_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct LowboyUser {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub access_token: Option<String>,
}

impl LowboyUser {
    pub async fn find(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        let record: LowboyUserRecord = lowboy_user::table.find(id).first(conn).await?;
        Self::from_record(&record, conn).await
    }

    pub async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Self> {
        let record: LowboyUserRecord = lowboy_user::table
            .filter(lowboy_user::username.eq(username))
            .first(conn)
            .await?;
        Self::from_record(&record, conn).await
    }

    pub async fn find_by_username_having_password(
        username: &str,
        conn: &mut Connection,
    ) -> QueryResult<Self> {
        let record: LowboyUserRecord = lowboy_user::table
            .filter(lowboy_user::username.eq(username))
            .filter(lowboy_user::password.is_not_null())
            .first(conn)
            .await?;
        Self::from_record(&record, conn).await
    }
}

#[async_trait::async_trait]
impl FromRecord<LowboyUserRecord> for LowboyUser {
    async fn from_record(record: &LowboyUserRecord, conn: &mut Connection) -> QueryResult<Self> {
        Self::from_record(record, conn).await
    }
}

impl LowboyUserTrait<LowboyUserRecord> for LowboyUser {
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

#[derive(PartialEq)]
pub enum Operation {
    Create = 0,
    Update = 1,
}

impl From<i64> for Operation {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::Create,
            1 => Self::Update,
            _ => unreachable!(),
        }
    }
}

impl<'a> NewLowboyUserRecord<'a> {
    pub async fn create_or_update(
        &self,
        conn: &mut Connection,
    ) -> QueryResult<(LowboyUserRecord, Operation)> {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                // @TODO can we do this in one query..?
                let operation = lowboy_user::table
                    .filter(lowboy_user::username.eq(self.username))
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map(Operation::from)?;

                let user: LowboyUserRecord = insert_into(lowboy_user::table)
                    .values(self)
                    .on_conflict(lowboy_user::email)
                    .do_update()
                    .set(lowboy_user::access_token.eq(excluded(lowboy_user::access_token)))
                    .get_result(conn)
                    .await?;

                Ok((user, operation))
            }
            .scope_boxed()
        })
        .await
    }
}

impl AuthUser for LowboyUserRecord {
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
