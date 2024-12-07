use diesel::dsl::{AsSelect, InnerJoin, Select};
use diesel::prelude::*;
use diesel::query_dsl::CompatibleType;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;
use lowboy::model::{FromRecord, LowboyUser, LowboyUserRecord, LowboyUserTrait, Model};
use lowboy::Connection;

use crate::schema::{lowboy_user, user};

pub trait DemoUser {
    fn name(&self) -> &String;
    fn avatar(&self) -> &Option<String>;
    fn byline(&self) -> &Option<String>;
}

#[derive(Debug, Default, Queryable, Selectable, Identifiable, Insertable, Associations)]
#[diesel(belongs_to(LowboyUserRecord, foreign_key = lowboy_user_id))]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserRecord {
    pub id: i32,
    pub lowboy_user_id: i32,
    pub name: String,
    pub avatar: Option<String>,
    pub byline: Option<String>,
}

impl UserRecord {
    pub fn create(lowboy_user_id: i32, name: &str) -> CreateUserRecord<'_> {
        CreateUserRecord::new(lowboy_user_id, name)
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

/// Convert from a `User` model into `UserRecord`
impl From<User> for UserRecord {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            lowboy_user_id: value.lowboy_user.id,
            name: value.name,
            avatar: value.avatar,
            byline: value.byline,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateUserRecord<'a> {
    pub lowboy_user_id: i32,
    pub name: &'a str,
    pub avatar: Option<&'a str>,
    pub byline: Option<&'a str>,
}

impl<'a> CreateUserRecord<'a> {
    /// Create a new `NewUserRecord` object
    pub fn new(lowboy_user_id: i32, name: &'a str) -> CreateUserRecord<'a> {
        Self {
            lowboy_user_id,
            name,
            ..Default::default()
        }
    }

    pub fn with_avatar(self, avatar: &'a str) -> CreateUserRecord<'a> {
        Self {
            avatar: Some(avatar),
            ..self
        }
    }

    pub fn with_byline(self, byline: &'a str) -> CreateUserRecord<'a> {
        Self {
            byline: Some(byline),
            ..self
        }
    }

    /// Create a new `user` in the database
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
    pub lowboy_user_id: Option<i32>,
    pub name: Option<&'a str>,
    pub avatar: Option<&'a str>,
    pub byline: Option<&'a str>,
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
            lowboy_user_id: Some(user.lowboy_user.id),
            name: Some(&user.name),
            avatar: user.avatar.as_deref(),
            byline: user.byline.as_deref(),
        }
    }

    pub fn from_record(record: &'a UserRecord) -> Self {
        Self {
            id: record.id,
            lowboy_user_id: Some(record.lowboy_user_id),
            name: Some(&record.name),
            avatar: record.avatar.as_deref(),
            byline: record.byline.as_deref(),
        }
    }

    pub fn with_lowboy_user_id(self, lowboy_user_id: i32) -> Self {
        Self {
            lowboy_user_id: Some(lowboy_user_id),
            ..self
        }
    }

    pub fn with_name(self, name: &'a str) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    pub fn with_avatar(self, avatar: &'a str) -> Self {
        Self {
            avatar: Some(avatar),
            ..self
        }
    }

    pub fn with_byline(self, byline: &'a str) -> Self {
        Self {
            byline: Some(byline),
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
    pub fn create_record(user_id: i32, content: &str) -> CreateUserRecord {
        CreateUserRecord::new(user_id, content)
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

#[derive(Clone, Debug, Default)]
pub struct User {
    pub id: i32,
    pub lowboy_user: LowboyUser,
    pub name: String,
    pub avatar: Option<String>,
    pub byline: Option<String>,
}

#[async_trait::async_trait]
impl Model for User {
    type Record = UserRecord;

    type RowSqlType = (user::SqlType, lowboy_user::SqlType);

    type Selection = (
        AsSelect<UserRecord, Sqlite>,
        AsSelect<LowboyUserRecord, Sqlite>,
    );

    type Query = Select<InnerJoin<user::table, lowboy_user::table>, Self::Selection>;

    fn query() -> Self::Query {
        user::table
            .inner_join(lowboy_user::table)
            .select((UserRecord::as_select(), LowboyUserRecord::as_select()))
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

impl CompatibleType<User, Sqlite> for <User as Model>::Selection {
    type SqlType = <User as Model>::RowSqlType;
}

impl Queryable<<User as Model>::RowSqlType, Sqlite> for User {
    type Row = (UserRecord, LowboyUserRecord);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (user_record, lowboy_user_record) = row;

        Ok(Self {
            id: user_record.id,
            lowboy_user: LowboyUser {
                username: lowboy_user_record.username,
                email: lowboy_user_record.email,
                ..Default::default()
            },
            name: user_record.name,
            avatar: user_record.avatar,
            byline: user_record.byline,
        })
    }
}

impl DemoUser for User {
    fn name(&self) -> &String {
        &self.name
    }

    fn avatar(&self) -> &Option<String> {
        &self.avatar
    }

    fn byline(&self) -> &Option<String> {
        &self.byline
    }
}

impl LowboyUserTrait<LowboyUserRecord> for User {
    fn id(&self) -> i32 {
        self.id
    }

    fn username(&self) -> &String {
        &self.lowboy_user.username
    }

    fn email(&self) -> &String {
        &self.lowboy_user.email
    }

    fn password(&self) -> &Option<String> {
        &self.lowboy_user.password
    }

    fn access_token(&self) -> &Option<String> {
        &self.lowboy_user.access_token
    }
}

#[async_trait::async_trait]
impl FromRecord<LowboyUserRecord> for User {
    async fn from_record(record: &LowboyUserRecord, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized,
    {
        Self::query()
            .filter(user::lowboy_user_id.eq(record.id))
            .first::<Self>(conn)
            .await
    }
}
