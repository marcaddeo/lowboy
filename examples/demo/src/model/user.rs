use std::collections::HashSet;

use diesel::dsl::{AsSelect, Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;
use lowboy::model::{Email, Model, Permission, Role, User as LowboyUser, UserModel};
use lowboy::Connection;

use crate::schema::{user, user_profile};

use super::UserProfileRecord;

#[derive(Clone, Debug)]
pub struct User {
    pub user: LowboyUser,
    pub profile: UserProfileRecord,
}

pub trait DemoUser {
    fn name(&self) -> &String;
    fn avatar(&self) -> Option<&String>;
    fn byline(&self) -> Option<&String>;
}

impl DemoUser for User {
    fn name(&self) -> &String {
        &self.profile.name
    }

    fn avatar(&self) -> Option<&String> {
        self.profile.avatar.as_ref()
    }

    fn byline(&self) -> Option<&String> {
        self.profile.byline.as_ref()
    }
}

#[diesel::dsl::auto_type]
pub fn user_from_clause() -> _ {
    let user_from_clause: <LowboyUser as Model>::FromClause = <LowboyUser as Model>::from_clause();

    user_from_clause.inner_join(user_profile::table)
}

#[diesel::dsl::auto_type]
pub fn user_select_clause() -> _ {
    let user_profile_select: AsSelect<UserProfileRecord, Sqlite> = UserProfileRecord::as_select();
    let user_as_select: <LowboyUser as Model>::SelectClause =
        <LowboyUser as Model>::select_clause();

    (user_profile_select, user_as_select)
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
    type Row = (UserProfileRecord, LowboyUser);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (profile_record, user) = row;

        Ok(Self {
            user,
            profile: profile_record,
        })
    }
}

#[async_trait::async_trait]
impl UserModel for User {
    fn id(&self) -> i32 {
        self.user.id
    }

    fn username(&self) -> &String {
        &self.user.username
    }

    fn email(&self) -> &Email {
        &self.user.email
    }

    fn password(&self) -> Option<&String> {
        self.user.password.as_ref()
    }

    fn access_token(&self) -> Option<&String> {
        self.user.access_token.as_ref()
    }

    async fn roles(&self, conn: &mut Connection) -> QueryResult<HashSet<Role>> {
        self.user.roles(conn).await
    }

    async fn permissions(&self, conn: &mut Connection) -> QueryResult<HashSet<Permission>> {
        self.user.permissions(conn).await
    }

    async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Option<Self>>
    where
        Self: Sized,
    {
        Self::query()
            .filter(user::username.eq(username))
            .first(conn)
            .await
            .optional()
    }
}
