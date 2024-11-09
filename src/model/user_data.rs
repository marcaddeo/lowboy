use crate::model::User;
use diesel::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, Default, Queryable, Identifiable, Associations, Serialize)]
#[diesel(belongs_to(User))]
#[diesel(table_name = crate::schema::user_data)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserData {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub avatar: Option<String>,
    pub byline: Option<String>,
}

impl UserData {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn avatar(&self) -> Option<&String> {
        self.avatar.as_ref()
    }

    pub fn byline(&self) -> Option<&String> {
        self.byline.as_ref()
    }
}

#[derive(Clone, Debug, Default, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::user_data)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewUserData<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub byline: Option<&'a str>,
    pub avatar: Option<&'a str>,
}

impl<'a> NewUserData<'a> {
    pub fn new(
        user_id: i32,
        name: &'a str,
        byline: Option<&'a str>,
        avatar: Option<&'a str>,
    ) -> Self {
        NewUserData {
            user_id,
            name,
            byline,
            avatar,
        }
    }
}
