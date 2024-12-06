use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use lowboy::model::{LowboyUser, LowboyUserRecord};
use lowboy::Connection;
use lowboy_record::prelude::*;

use super::User;
use crate::model::UserRecord;
use crate::schema::{lowboy_user, post, user};

#[apply(lowboy_record!)]
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
#[diesel(table_name = crate::schema::post)]
#[diesel(belongs_to(UserRecord, foreign_key = user_id))]
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
            .inner_join(user::table.inner_join(lowboy_user::table))
            .limit(limit.unwrap_or(100))
            .order_by(post::id.desc())
            .load::<(PostRecord, (UserRecord, LowboyUserRecord))>(conn)
            .await?;
        let mut posts = vec![];

        for (post_record, (user_record, lowboy_user_record)) in &records {
            let post = Post {
                id: post_record.id,
                user: User {
                    id: user_record.id,
                    lowboy_user: LowboyUser {
                        id: lowboy_user_record.id,
                        username: lowboy_user_record.username.clone(),
                        email: lowboy_user_record.email.clone(),
                        password: None,
                        access_token: None,
                    },
                    name: user_record.name.clone(),
                    avatar: user_record.avatar.clone(),
                    byline: user_record.byline.clone(),
                },
                content: post_record.content.clone(),
            };

            posts.push(post);
        }

        Ok(posts)
    }
}
