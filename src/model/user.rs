use crate::model::{UserData, UserDataRecord};
use crate::schema::{user, user_data};
use crate::Connection;
use axum_login::AuthUser;
use derive_masked::DebugMasked;
use diesel::upsert::excluded;
use diesel::{insert_into, prelude::*};
use diesel::{QueryResult, Selectable};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use lowboy_record::prelude::*;

use super::NewUserDataRecord;

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
    pub data: HasOne<UserData>,
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

impl<'a> NewUserRecord<'a> {
    pub async fn create_or_update(
        &self,
        name: &'a str,
        byline: Option<&'a str>,
        avatar: Option<&'a str>,
        conn: &mut Connection,
    ) -> QueryResult<UserRecord> {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                let user: UserRecord = insert_into(user::table)
                    .values(self)
                    .on_conflict(user::email)
                    .do_update()
                    .set(user::access_token.eq(excluded(user::access_token)))
                    .get_result(conn)
                    .await?;
                let data = NewUserDataRecord::new(user.id, name)
                    .with_avatar(avatar)
                    .with_byline(byline);
                insert_into(user_data::table)
                    .values(data.clone())
                    .on_conflict(user_data::user_id)
                    .do_nothing()
                    .execute(conn)
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
