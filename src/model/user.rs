use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::associations::HasTable;
use diesel::dsl::{AsSelect, Select};
use diesel::prelude::*;
use diesel::query_dsl::CompatibleType;
use diesel::sqlite::Sqlite;
use diesel::upsert::excluded;
use diesel::{insert_into, OptionalExtension, QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use gravatar_api::avatars as gravatars;

use crate::schema::lowboy_user;
use crate::Connection;

use super::Model;

#[derive(Clone, Debug, Default)]
pub struct LowboyUser {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub access_token: Option<String>,
}

impl LowboyUser {
    pub async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Self> {
        Self::query()
            .filter(lowboy_user::username.eq(username))
            .first::<Self>(conn)
            .await
    }

    pub async fn find_by_username_having_password(
        username: &str,
        conn: &mut Connection,
    ) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(lowboy_user::username.eq(username))
            .filter(lowboy_user::password.is_not_null())
            .first::<Self>(conn)
            .await
            .optional()
    }
}

pub trait LowboyUserTrait<T>: Model + FromRecord<T> {
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

#[async_trait::async_trait]
impl Model for LowboyUser {
    type Record = LowboyUserRecord;

    type RowSqlType = (lowboy_user::SqlType,);

    type Selection = (AsSelect<LowboyUserRecord, Sqlite>,);

    type Query = Select<lowboy_user::table, Self::Selection>;

    fn query() -> Self::Query {
        Self::Record::table().select((Self::Record::as_select(),))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized,
    {
        Self::query()
            .filter(lowboy_user::id.eq(id))
            .first::<Self>(conn)
            .await
    }
}

impl CompatibleType<LowboyUser, Sqlite> for <LowboyUser as Model>::Selection {
    type SqlType = <LowboyUser as Model>::RowSqlType;
}

impl Queryable<<LowboyUser as Model>::RowSqlType, Sqlite> for LowboyUser {
    type Row = (LowboyUserRecord,);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (lowboy_user_record,) = row;

        Ok(Self {
            id: lowboy_user_record.id,
            username: lowboy_user_record.username,
            email: lowboy_user_record.email,
            password: lowboy_user_record.password,
            access_token: lowboy_user_record.access_token,
        })
    }
}

#[async_trait::async_trait]
pub trait FromRecord<T> {
    async fn from_record(record: &T, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl FromRecord<LowboyUserRecord> for LowboyUser {
    async fn from_record(record: &LowboyUserRecord, _conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized,
    {
        let record = record.clone();
        Ok(Self {
            id: record.id,
            username: record.username,
            email: record.email,
            password: record.password,
            access_token: record.access_token,
        })
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

impl<'a> CreateLowboyUserRecord<'a> {
    pub async fn save_or_update(
        &self,
        conn: &mut Connection,
    ) -> QueryResult<(LowboyUserRecord, Operation)> {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                // @TODO can we do this in one query..?
                let operation = LowboyUser::query()
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

// @note the rest of this file is to eventually be generated using lowboy_record!
#[derive(Clone, DebugMasked, Default, Queryable, Selectable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::schema::lowboy_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct LowboyUserRecord {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub access_token: Option<String>,
}

impl LowboyUserRecord {
    pub fn create<'a>(username: &'a str, email: &'a str) -> CreateLowboyUserRecord<'a> {
        CreateLowboyUserRecord::new(username, email)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<LowboyUserRecord> {
        lowboy_user::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdateLowboyUserRecord {
        UpdateLowboyUserRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(lowboy_user::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `User` model into `LowboyUserRecord`
impl From<LowboyUser> for LowboyUserRecord {
    fn from(value: LowboyUser) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email: value.email,
            password: value.password,
            access_token: value.access_token,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::lowboy_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateLowboyUserRecord<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: Option<&'a str>,
    pub access_token: Option<&'a str>,
}

impl<'a> CreateLowboyUserRecord<'a> {
    /// Create a new `NewLowboyUserRecord` object
    pub fn new(username: &'a str, email: &'a str) -> CreateLowboyUserRecord<'a> {
        Self {
            username,
            email,
            ..Default::default()
        }
    }

    pub fn with_password(self, password: &'a str) -> CreateLowboyUserRecord<'a> {
        Self {
            password: Some(password),
            ..self
        }
    }

    pub fn with_access_token(self, access_token: &'a str) -> CreateLowboyUserRecord<'a> {
        Self {
            access_token: Some(access_token),
            ..self
        }
    }

    /// Create a new `user` in the database
    pub async fn save(&self, conn: &mut Connection) -> QueryResult<LowboyUserRecord> {
        diesel::insert_into(crate::schema::lowboy_user::table)
            .values(self)
            .returning(crate::schema::lowboy_user::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::lowboy_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateLowboyUserRecord<'a> {
    pub id: i32,
    pub username: &'a str,
    pub email: &'a str,
    pub password: Option<&'a str>,
    pub access_token: Option<&'a str>,
}

impl<'a> UpdateLowboyUserRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_lowboy_user(lowboy_user: &'a LowboyUser) -> Self {
        Self {
            id: lowboy_user.id,
            username: &lowboy_user.username,
            email: &lowboy_user.email,
            password: lowboy_user.password.as_deref(),
            access_token: lowboy_user.access_token.as_deref(),
        }
    }

    pub fn from_record(record: &'a LowboyUserRecord) -> Self {
        Self {
            id: record.id,
            username: &record.username,
            email: &record.email,
            password: record.password.as_deref(),
            access_token: record.access_token.as_deref(),
        }
    }

    pub fn with_username(self, username: &'a str) -> Self {
        Self { username, ..self }
    }

    pub fn with_email(self, email: &'a str) -> Self {
        Self { email, ..self }
    }

    pub fn with_password(self, password: &'a str) -> Self {
        Self {
            password: Some(password),
            ..self
        }
    }

    pub fn with_access_token(self, access_token: &'a str) -> Self {
        Self {
            access_token: Some(access_token),
            ..self
        }
    }

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<LowboyUserRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::lowboy_user::all_columns)
            .get_result(conn)
            .await
    }
}

impl LowboyUser {
    pub fn create_record<'a>(username: &'a str, email: &'a str) -> CreateLowboyUserRecord<'a> {
        CreateLowboyUserRecord::new(username, email)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<LowboyUserRecord> {
        LowboyUserRecord::read(id, conn).await
    }

    pub fn update_record(&self) -> UpdateLowboyUserRecord {
        UpdateLowboyUserRecord::from_lowboy_user(self)
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        LowboyUserRecord::from(self).delete(conn).await
    }
}
