use super::User;
use crate::model::UserRecord;
use crate::schema::post;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use lowboy::{model::LowboyUserRecord, Connection};
use lowboy_record::prelude::*;

#[apply(lowboy_record!)]
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
#[diesel(table_name = crate::schema::post)]
#[diesel(belongs_to(LowboyUserRecord, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Post {
    pub id: i32,
    pub user: Related<User>,
    pub content: String,
}

impl Post {
    pub async fn find(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        let record: PostRecord = post::table.find(id).first(conn).await?;
        Self::from_record(&record, conn).await
    }

    pub async fn list(conn: &mut Connection, limit: Option<i64>) -> QueryResult<Vec<Self>> {
        let records = post::table
            .select(PostRecord::as_select())
            .limit(limit.unwrap_or(100))
            .load(conn)
            .await?;
        let mut posts = vec![];

        for record in &records {
            posts.push(Self::from_record(record, conn).await?);
        }

        Ok(posts)
    }
}
