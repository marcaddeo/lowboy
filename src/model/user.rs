use std::collections::HashSet;

use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::dsl::{AsSelect, InnerJoin, LeftJoin, Nullable, Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::result::Error::{DeserializationError, NotFound};
use diesel::result::UnexpectedNullError;
use diesel::sql_types::{Integer, Text};
use diesel::sqlite::Sqlite;
use diesel::{OptionalExtension, QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use gravatar_api::avatars as gravatars;

use crate::schema::{email, lowboy_user, permission, role, role_permission, user_role};
use crate::Connection;

use super::{Email, EmailRecord, Model, Permission, Role, UnverifiedEmail};

// @note: don't really love this solution, but GROUP BY with diesel doesn't seem to be able to work
// across crates so that's problematic.
pub trait AssumeNullIsNotFoundExtension<T> {
    fn assume_null_is_not_found(self) -> QueryResult<T>;
}

impl<T> AssumeNullIsNotFoundExtension<T> for QueryResult<T> {
    fn assume_null_is_not_found(self) -> QueryResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(DeserializationError(e)) if e.is::<UnexpectedNullError>() => {
                tracing::debug!(
                    "assuming null is not found for {type_name}: {e}",
                    type_name = std::any::type_name::<T>()
                );
                Err(NotFound)
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LowboyUser {
    pub id: i32,
    pub username: String,
    pub email: Email,
    pub password: Option<String>,
    pub access_token: Option<String>,
    pub roles: HashSet<Role>,
    pub permissions: HashSet<Permission>,
}

impl LowboyUser {
    pub async fn new(
        username: &str,
        email: &str,
        password: Option<&str>,
        access_token: Option<&str>,
        conn: &mut Connection,
    ) -> QueryResult<Self> {
        conn.transaction(|conn| {
            async move {
                let user = CreateLowboyUserRecord {
                    username,
                    password,
                    access_token,
                }
                .save(conn)
                .await?;

                UnverifiedEmail::new(user.id, email, conn).await?;

                Role::find_by_name("unverified", conn)
                    .await?
                    .expect("unverified role should exist")
                    .assign(user.id, conn)
                    .await?;

                <Self as Model>::load(user.id, conn).await
            }
            .scope_boxed()
        })
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
            .assume_null_is_not_found()
            .optional()
    }
}

#[async_trait::async_trait]
pub trait LowboyUserTrait: Model + FromLowboyUser {
    fn id(&self) -> i32;
    fn username(&self) -> &String;
    fn email(&self) -> &Email;
    fn password(&self) -> Option<&String>;
    fn access_token(&self) -> Option<&String>;
    fn roles(&self) -> &HashSet<Role>;
    fn permissions(&self) -> &HashSet<Permission>;
    fn gravatar(&self) -> String {
        gravatars::Avatar::builder(&self.email().address)
            .size(256)
            .default(gravatars::Default::MysteryPerson)
            .rating(gravatars::Rating::Pg)
            .build()
            .image_url()
            .to_string()
    }

    async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Option<Self>>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl LowboyUserTrait for LowboyUser {
    fn id(&self) -> i32 {
        self.id
    }

    fn username(&self) -> &String {
        &self.username
    }

    fn email(&self) -> &Email {
        &self.email
    }

    fn password(&self) -> Option<&String> {
        self.password.as_ref()
    }

    fn access_token(&self) -> Option<&String> {
        self.access_token.as_ref()
    }

    fn roles(&self) -> &HashSet<Role> {
        &self.roles
    }

    fn permissions(&self) -> &HashSet<Permission> {
        &self.permissions
    }

    async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(lowboy_user::username.eq(username))
            .first::<Self>(conn)
            .await
            .assume_null_is_not_found()
            .optional()
    }
}

define_sql_function! {
    fn group_concat(val: Text, separator: Text) -> Text;
}

define_sql_function! {
    fn json_group_array(val: Text) -> Text;
}

// @TODO i believe Diesel will be adding general support for json_object eventually, so this should
// be a temporary solution
define_sql_function! {
    #[sql_name = "json_object"]
    fn role_record_json(a: Text, b: Integer, c: Text, d: Text) -> Text;
}

define_sql_function! {
    #[sql_name = "json_object"]
    fn permission_record_json(a: Text, b: diesel::sql_types::Nullable<Integer>, c: Text, d: diesel::sql_types::Nullable<Text>) -> Text;
}

#[async_trait::async_trait]
impl Model for LowboyUser {
    type RowSqlType = (
        AsSelect<LowboyUserRecord, Sqlite>,
        AsSelect<EmailRecord, Sqlite>,
        SqlTypeOf<
            json_group_array<role_record_json<&'static str, role::id, &'static str, role::name>>,
        >,
        SqlTypeOf<
            json_group_array<
                permission_record_json<
                    &'static str,
                    Nullable<permission::id>,
                    &'static str,
                    Nullable<permission::name>,
                >,
            >,
        >,
    );
    type Selection = (
        AsSelect<LowboyUserRecord, Sqlite>,
        AsSelect<EmailRecord, Sqlite>,
        json_group_array<role_record_json<&'static str, role::id, &'static str, role::name>>,
        json_group_array<
            permission_record_json<
                &'static str,
                Nullable<permission::id>,
                &'static str,
                Nullable<permission::name>,
            >,
        >,
    );
    type Query = Select<
        InnerJoin<
            InnerJoin<lowboy_user::table, email::table>,
            InnerJoin<
                user_role::table,
                LeftJoin<role::table, LeftJoin<role_permission::table, permission::table>>,
            >,
        >,
        Self::Selection,
    >;

    fn query() -> Self::Query {
        lowboy_user::table
            .inner_join(email::table)
            .inner_join(user_role::table.inner_join(
                role::table.left_join(role_permission::table.left_join(permission::table)),
            ))
            .select((
                LowboyUserRecord::as_select(),
                EmailRecord::as_select(),
                json_group_array(role_record_json("id", role::id, "name", role::name)),
                json_group_array(permission_record_json(
                    "id",
                    permission::id.nullable(),
                    "name",
                    permission::name.nullable(),
                )),
            ))
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

impl Queryable<<LowboyUser as Model>::RowSqlType, Sqlite> for LowboyUser {
    type Row = (LowboyUserRecord, EmailRecord, String, String);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (lowboy_user_record, email_record, roles, permissions) = row;

        Ok(Self {
            id: lowboy_user_record.id,
            username: lowboy_user_record.username,
            email: email_record.into(),
            password: lowboy_user_record.password,
            access_token: lowboy_user_record.access_token,
            roles: serde_json::from_str(&roles).unwrap_or_default(),
            permissions: serde_json::from_str(&permissions).unwrap_or_default(),
        })
    }
}

#[async_trait::async_trait]
pub trait FromLowboyUser {
    async fn from_lowboy_user(user: &LowboyUser, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl FromLowboyUser for LowboyUser {
    async fn from_lowboy_user(user: &LowboyUser, _conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized,
    {
        Ok(user.clone())
    }
}

impl AuthUser for LowboyUser {
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
    pub password: Option<String>,
    pub access_token: Option<String>,
}

impl LowboyUserRecord {
    pub fn create(username: &str) -> CreateLowboyUserRecord {
        CreateLowboyUserRecord::new(username)
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
    pub password: Option<&'a str>,
    pub access_token: Option<&'a str>,
}

impl<'a> CreateLowboyUserRecord<'a> {
    /// Create a new `NewLowboyUserRecord` object
    pub fn new(username: &'a str) -> CreateLowboyUserRecord<'a> {
        Self {
            username,
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
            password: lowboy_user.password.as_deref(),
            access_token: lowboy_user.access_token.as_deref(),
        }
    }

    pub fn from_record(record: &'a LowboyUserRecord) -> Self {
        Self {
            id: record.id,
            username: &record.username,
            password: record.password.as_deref(),
            access_token: record.access_token.as_deref(),
        }
    }

    pub fn with_username(self, username: &'a str) -> Self {
        Self { username, ..self }
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
    pub fn create_record(username: &str) -> CreateLowboyUserRecord {
        CreateLowboyUserRecord::new(username)
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
