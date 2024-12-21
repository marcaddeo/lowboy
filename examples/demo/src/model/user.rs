use std::collections::HashSet;

use diesel::dsl::{AsSelect, InnerJoin, Select};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;
use lowboy::model::{
    AssumeNullIsNotFoundExtension as _, Email, Model, Permission, Role, User as LowboyUser,
    UserModel,
};
use lowboy::schema::user;
use lowboy::Connection;

use crate::schema::user_profile;

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

#[async_trait::async_trait]
impl Model for User {
    type RowSqlType = (
        AsSelect<UserProfileRecord, Sqlite>,
        <LowboyUser as Model>::RowSqlType,
    );
    type Selection = (
        AsSelect<UserProfileRecord, Sqlite>,
        <LowboyUser as Model>::Selection,
    );
    type Query = Select<
        InnerJoin<<LowboyUser as Model>::Query, user_profile::table>,
        (
            AsSelect<UserProfileRecord, Sqlite>,
            <LowboyUser as Model>::Selection,
        ),
    >;

    fn query() -> Self::Query {
        LowboyUser::query()
            .inner_join(user_profile::table)
            .select((UserProfileRecord::as_select(), LowboyUser::query().select.0))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized,
    {
        Self::query()
            .filter(user::id.eq(id))
            .first::<Self>(conn)
            .await
    }
}

impl Queryable<<User as Model>::RowSqlType, Sqlite> for User {
    type Row = (
        UserProfileRecord,
        <LowboyUser as Queryable<<LowboyUser as Model>::RowSqlType, Sqlite>>::Row,
    );

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (profile_record, row) = row;

        Ok(Self {
            user: LowboyUser::build(row)?,
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

    fn roles(&self) -> &HashSet<Role> {
        &self.user.roles
    }

    fn permissions(&self) -> &HashSet<Permission> {
        &self.user.permissions
    }

    async fn find_by_username(username: &str, conn: &mut Connection) -> QueryResult<Option<Self>>
    where
        Self: Sized,
    {
        Self::query()
            .filter(user::username.eq(username))
            .first::<Self>(conn)
            .await
            .assume_null_is_not_found()
            .optional()
    }
}
