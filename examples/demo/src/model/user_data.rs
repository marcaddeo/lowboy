use crate::model::UserRecord;
use crate::Connection;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use lowboy_record::prelude::*;

#[apply(lowboy_record!)]
#[derive(Debug, Default, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(UserRecord, foreign_key = user_id))]
#[diesel(table_name = crate::schema::user_data)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserData {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub avatar: Option<String>,
    pub byline: Option<String>,
}
