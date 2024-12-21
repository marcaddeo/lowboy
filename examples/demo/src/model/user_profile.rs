use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use lowboy::model::UserRecord;
use lowboy::Connection;

use crate::schema::user_profile;

// @note the rest of this file is to eventually be generated using lowboy_record!
#[derive(Clone, Debug, Default, Queryable, Selectable, Identifiable, Insertable, Associations)]
#[diesel(belongs_to(UserRecord, foreign_key = user_id))]
#[diesel(table_name = crate::schema::user_profile)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserProfileRecord {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub avatar: Option<String>,
    pub byline: Option<String>,
}

impl UserProfileRecord {
    pub fn create(user_id: i32, name: &str) -> CreateUserProfileRecord<'_> {
        CreateUserProfileRecord::new(user_id, name)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<UserProfileRecord> {
        user_profile::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdateUserProfileRecord {
        UpdateUserProfileRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(user_profile::table.find(self.id))
            .execute(conn)
            .await
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::user_profile)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateUserProfileRecord<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub avatar: Option<&'a str>,
    pub byline: Option<&'a str>,
}

impl<'a> CreateUserProfileRecord<'a> {
    /// Create a new `NewUserProfileRecord` object
    pub fn new(user_id: i32, name: &'a str) -> CreateUserProfileRecord<'a> {
        Self {
            user_id,
            name,
            ..Default::default()
        }
    }

    pub fn with_avatar(self, avatar: &'a str) -> CreateUserProfileRecord<'a> {
        Self {
            avatar: Some(avatar),
            ..self
        }
    }

    pub fn with_byline(self, byline: &'a str) -> CreateUserProfileRecord<'a> {
        Self {
            byline: Some(byline),
            ..self
        }
    }

    /// Create a new `user` in the database
    pub async fn save(&self, conn: &mut Connection) -> QueryResult<UserProfileRecord> {
        diesel::insert_into(crate::schema::user_profile::table)
            .values(self)
            .returning(crate::schema::user_profile::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::user_profile)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateUserProfileRecord<'a> {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: Option<&'a str>,
    pub avatar: Option<&'a str>,
    pub byline: Option<&'a str>,
}

impl<'a> UpdateUserProfileRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_record(record: &'a UserProfileRecord) -> Self {
        Self {
            id: record.id,
            user_id: Some(record.user_id),
            name: Some(&record.name),
            avatar: record.avatar.as_deref(),
            byline: record.byline.as_deref(),
        }
    }

    pub fn with_user_id(self, user_id: i32) -> Self {
        Self {
            user_id: Some(user_id),
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

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<UserProfileRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::user_profile::all_columns)
            .get_result(conn)
            .await
    }
}
