#![allow(dead_code)]
#![allow(unused_variables)]

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::RunQueryDsl;
use lowboy_record::prelude::*;

pub type Connection = SyncConnectionWrapper<SqliteConnection>;

pub mod schema {
    use diesel::table;

    table! {
        user_data (id) {
            id -> Integer,
            user_id -> Integer,
            avatar -> Nullable<Text>,
        }
    }

    table! {
        user (id) {
            id -> Integer,
            name -> Text,
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
    #[apply(lowboy_record!)]
    #[derive(Debug, Default, Queryable, Identifiable, Associations)]
    #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
    #[diesel(table_name = crate::schema::user_data)]
    #[diesel(check_for_backend(diesel::sqlite::Sqlite))]
    pub struct UserData {
        pub id: i32,
        pub user_id: i32,
        pub avatar: Option<String>,
    }

    #[apply(lowboy_record!)]
    #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable)]
    #[diesel(table_name = crate::schema::user)]
    pub struct User {
        pub id: i32,
        pub name: String,
        pub data: HasOne<UserData>,
        pub posts: Related<Vec<Post>>,
    }

    #[apply(lowboy_record!)]
    #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
    #[diesel(table_name = crate::schema::post)]
    #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
    pub struct Post {
        pub id: i32,
        pub user: Related<User>,
        pub content: String,
    }

    #[apply(lowboy_record!)]
    #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
    #[diesel(table_name = crate::schema::comment)]
    #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
    #[diesel(belongs_to(PostRecord, foreign_key = post_id))]
    pub struct Comment {
        pub id: i32,
        pub user: Related<User>,
        pub post: Related<Post>,
        pub content: String,
    }

    let record = Post::new_record(123, "some content");

    assert_eq!(record.user_id, 123);
    assert_eq!(record.content, "some content");
}
