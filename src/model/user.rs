use std::ops::Deref;

use super::user_data::UserData;
use crate::app::Connection;
use crate::model::user_data::NewUserData;
use crate::schema::{user, user_data};
use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::insert_into;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel::{QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::Serialize;

#[derive(
    Clone, DebugMasked, Default, Queryable, Selectable, AsChangeset, Identifiable, Serialize,
)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    #[masked]
    pub password: Option<String>,
    #[masked]
    pub access_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserWithData {
    #[serde(flatten)]
    pub user: User,
    pub data: UserData,
}

impl Deref for UserWithData {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

impl UserWithData {
    pub async fn from_user(user: User, conn: &mut Connection) -> QueryResult<Self> {
        let data = user.data(conn).await?;
        Ok(Self { user, data })
    }
}

impl User {
    pub async fn find(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        user::table.find(id).first(conn).await
    }

    pub async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Self> {
        user::table
            .filter(user::username.eq(username))
            .first(conn)
            .await
    }

    pub async fn find_by_username_having_password(
        username: &str,
        conn: &mut Connection,
    ) -> QueryResult<Self> {
        user::table
            .filter(user::username.eq(username))
            .filter(user::password.is_not_null())
            .first(conn)
            .await
    }

    pub async fn data(&self, conn: &mut Connection) -> QueryResult<UserData> {
        UserData::belonging_to(self)
            .select(user_data::table::all_columns())
            .first(conn)
            .await
    }
}

#[derive(Clone, Default, Insertable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: Option<&'a str>,
    pub access_token: Option<&'a str>,
}

impl std::fmt::Debug for NewUser<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewUser")
            .field("username", &self.username)
            .field("email", &self.email)
            .field("password", &"[redacted]")
            .field("access_token", &"[redacted]")
            .finish()
    }
}

impl<'a> NewUser<'a> {
    pub fn new(
        username: &'a str,
        email: &'a str,
        password: Option<&'a str>,
        access_token: Option<&'a str>,
    ) -> Self {
        NewUser {
            username,
            email,
            password,
            access_token,
        }
    }

    pub async fn create_or_update(
        &self,
        name: &'a str,
        byline: Option<&'a str>,
        avatar: Option<&'a str>,
        conn: &mut Connection,
    ) -> QueryResult<User> {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                let user: User = insert_into(user::table)
                    .values(self)
                    .on_conflict(user::email)
                    .do_update()
                    .set(user::access_token.eq(excluded(user::access_token)))
                    .get_result(conn)
                    .await?;

                let new_user_data = NewUserData {
                    user_id: user.id,
                    name,
                    byline,
                    avatar,
                };

                insert_into(user_data::table)
                    .values(new_user_data.clone())
                    .on_conflict(user_data::user_id)
                    .do_update()
                    .set(&new_user_data)
                    .execute(conn)
                    .await?;

                Ok(user)
            }
            .scope_boxed()
        })
        .await
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
