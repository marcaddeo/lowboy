use std::collections::HashSet;

use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::dsl::{AsSelect, Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel::{OptionalExtension, QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use gravatar_api::avatars as gravatars;

use crate::schema::{email, permission, role, role_permission, user, user_role};
use crate::Connection;

use super::{Email, Model, Permission, PermissionRecord, Role, RoleRecord, UnverifiedEmail};

pub trait HasPermission: Sized + IntoIterator<Item = Permission> {
    fn has_permission(self, permission: &str) -> bool {
        self.into_iter().any(|p| p.name == permission)
    }
}

pub trait HasRole: Sized + IntoIterator<Item = Role> {
    fn has_role(self, role: &str) -> bool {
        self.into_iter().any(|p| p.name == role)
    }
}

impl HasRole for HashSet<Role> {}
impl HasPermission for HashSet<Permission> {}

#[derive(Clone, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: Email,
    pub password: Option<String>,
    pub access_token: Option<String>,
    pub roles: HashSet<Role>,
    pub permissions: HashSet<Permission>,
}

impl User {
    pub async fn new(
        username: &str,
        email: &str,
        password: Option<&str>,
        access_token: Option<&str>,
        conn: &mut Connection,
    ) -> QueryResult<Self> {
        conn.transaction(|conn| {
            async move {
                let user = CreateUserRecord {
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
            .filter(user::username.eq(username))
            .filter(user::password.is_not_null())
            .first(conn)
            .await
            .optional()
    }
}

#[async_trait::async_trait]
pub trait UserModel: Model
where
    Self: Sized,
{
    fn id(&self) -> i32;
    fn username(&self) -> &String;
    fn email(&self) -> &Email;
    fn password(&self) -> Option<&String>;
    fn access_token(&self) -> Option<&String>;
    fn gravatar(&self) -> String {
        gravatars::Avatar::builder(&self.email().address)
            .size(256)
            .default(gravatars::Default::MysteryPerson)
            .rating(gravatars::Rating::Pg)
            .build()
            .image_url()
            .to_string()
    }

    async fn roles(&self, conn: &mut Connection) -> QueryResult<HashSet<Role>>;
    async fn permissions(&self, conn: &mut Connection) -> QueryResult<HashSet<Permission>>;

    async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Option<Self>>;
}

#[async_trait::async_trait]
impl UserModel for User {
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

    async fn roles(&self, conn: &mut Connection) -> QueryResult<HashSet<Role>> {
        let roles = role::table
            .inner_join(user_role::table)
            .filter(user_role::user_id.eq(self.id))
            .select(RoleRecord::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(Role::from);

        Ok(HashSet::from_iter(roles))
    }

    async fn permissions(&self, conn: &mut Connection) -> QueryResult<HashSet<Permission>> {
        let permissions = permission::table
            .inner_join(role_permission::table.inner_join(role::table.inner_join(user_role::table)))
            .filter(user_role::user_id.eq(self.id))
            .select(PermissionRecord::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(Permission::from);

        Ok(HashSet::from_iter(permissions))
    }

    async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(user::username.eq(username))
            .first::<Self>(conn)
            .await
            .optional()
    }
}

#[diesel::dsl::auto_type]
pub fn user_from_clause() -> _ {
    user::table.inner_join(email::table)
}

#[diesel::dsl::auto_type]
pub fn user_select_clause() -> _ {
    let user_as_select: AsSelect<UserRecord, Sqlite> = UserRecord::as_select();
    let email_as_select: <Email as Model>::SelectClause = <Email as Model>::select_clause();

    (user_as_select, email_as_select)
}

#[async_trait::async_trait]
impl Model for User {
    type RowSqlType = SqlTypeOf<Self::SelectClause>;
    type SelectClause = user_select_clause;
    type FromClause = user_from_clause;
    type Query = Select<Self::FromClause, Self::SelectClause>;

    fn query() -> Self::Query {
        Self::from_clause().select(Self::select_clause())
    }

    fn from_clause() -> Self::FromClause {
        user_from_clause()
    }

    fn select_clause() -> Self::SelectClause {
        user_select_clause()
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query().filter(user::id.eq(id)).first(conn).await
    }
}

impl Selectable<Sqlite> for User {
    type SelectExpression = <Self as Model>::SelectClause;

    fn construct_selection() -> Self::SelectExpression {
        Self::select_clause()
    }
}

impl Queryable<<User as Model>::RowSqlType, Sqlite> for User {
    type Row = (UserRecord, Email);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (user_record, email) = row;

        Ok(Self {
            id: user_record.id,
            username: user_record.username,
            email,
            password: user_record.password,
            access_token: user_record.access_token,
            roles: HashSet::new(), // @TODO
            permissions: HashSet::new(),
        })
    }
}

impl AuthUser for User {
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
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserRecord {
    pub id: i32,
    pub username: String,
    pub password: Option<String>,
    pub access_token: Option<String>,
}

impl UserRecord {
    pub fn create(username: &str) -> CreateUserRecord {
        CreateUserRecord::new(username)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<UserRecord> {
        user::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdateUserRecord {
        UpdateUserRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(user::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `User` model into `LowboyUserRecord`
impl From<User> for UserRecord {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
            password: value.password,
            access_token: value.access_token,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateUserRecord<'a> {
    pub username: &'a str,
    pub password: Option<&'a str>,
    pub access_token: Option<&'a str>,
}

impl<'a> CreateUserRecord<'a> {
    pub fn new(username: &'a str) -> CreateUserRecord<'a> {
        Self {
            username,
            ..Default::default()
        }
    }

    pub fn with_password(self, password: &'a str) -> CreateUserRecord<'a> {
        Self {
            password: Some(password),
            ..self
        }
    }

    pub fn with_access_token(self, access_token: &'a str) -> CreateUserRecord<'a> {
        Self {
            access_token: Some(access_token),
            ..self
        }
    }

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<UserRecord> {
        diesel::insert_into(crate::schema::user::table)
            .values(self)
            .returning(crate::schema::user::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateUserRecord<'a> {
    pub id: i32,
    pub username: &'a str,
    pub password: Option<&'a str>,
    pub access_token: Option<&'a str>,
}

impl<'a> UpdateUserRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_user(user: &'a User) -> Self {
        Self {
            id: user.id,
            username: &user.username,
            password: user.password.as_deref(),
            access_token: user.access_token.as_deref(),
        }
    }

    pub fn from_record(record: &'a UserRecord) -> Self {
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

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<UserRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::user::all_columns)
            .get_result(conn)
            .await
    }
}

impl User {
    pub fn create_record(username: &str) -> CreateUserRecord {
        CreateUserRecord::new(username)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<UserRecord> {
        UserRecord::read(id, conn).await
    }

    pub fn update_record(&self) -> UpdateUserRecord {
        UpdateUserRecord::from_user(self)
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        UserRecord::from(self).delete(conn).await
    }
}
