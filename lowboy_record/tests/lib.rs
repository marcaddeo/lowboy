#![allow(dead_code)]
#![allow(unused_variables)]

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::RunQueryDsl;
use lowboy_record::lowboy_record;

pub type Connection = SyncConnectionWrapper<SqliteConnection>;

pub mod schema {
    use diesel::table;

    table! {
        user (id) {
            id -> Integer,
            name -> Text,
            avatar -> Nullable<Text>,
        }
    }

    table! {
        post (id) {
            id -> Integer,
            user_id -> Integer,
            content -> Text,
        }
    }

    table! {
        comment (id) {
            id -> Integer,
            user_id -> Integer,
            post_id -> Integer,
            content -> Text,
        }
    }
}

#[test]
fn lowboy_record_works() {
    lowboy_record!(
        #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable)]
        #[diesel(table_name = crate::schema::user)]
        pub struct User {
            id: i32,
            name: String,
            avatar: Option<String>,
            posts: Related<Vec<Post>>,
        }
    );

    lowboy_record!(
        #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
        #[diesel(table_name = crate::schema::post)]
        #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
        pub struct Post {
            id: i32,
            user: Related<User>,
            content: String,
        }
    );
    lowboy_record!(
        #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
        #[diesel(table_name = crate::schema::comment)]
        #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
        #[diesel(belongs_to(PostRecord, foreign_key = post_id))]
        pub struct Comment {
            id: i32,
            user: Related<User>,
            post: Related<Post>,
            content: String,
        }
    );

    let record = Post::new_record(123, "some content");

    assert_eq!(record.user_id, 123);
    assert_eq!(record.content, "some content");
}
