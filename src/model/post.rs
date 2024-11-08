use super::User;
use super::UserWithData;
use crate::app::Connection;
use crate::schema::post;
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;

#[derive(
    Clone, Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations, Serialize,
)]
#[diesel(table_name = crate::schema::post)]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub content: String,
}

#[derive(Serialize)]
pub struct PostWithAuthor {
    #[serde(flatten)]
    pub post: Post,
    pub author: UserWithData,
}

impl PostWithAuthor {
    pub async fn from_post(post: Post, conn: &mut Connection) -> QueryResult<Self> {
        let user = post.user(conn).await?;
        let author = UserWithData::from_user(user, conn).await?;
        Ok(Self { post, author })
    }

    pub async fn from_posts(posts: Vec<Post>, conn: &mut Connection) -> QueryResult<Vec<Self>> {
        let mut posts_with_author: Vec<PostWithAuthor> = Vec::new();

        for post in posts {
            let post_with_author = PostWithAuthor::from_post(post, conn).await?;
            posts_with_author.push(post_with_author);
        }

        Ok(posts_with_author)
    }

    pub fn content(&self) -> &str {
        self.post.content()
    }
}

impl Post {
    pub async fn find(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        post::table.find(id).first(conn).await
    }

    pub async fn list(conn: &mut Connection, limit: Option<i64>) -> QueryResult<Vec<Self>> {
        let posts = post::table
            .select(Post::as_select())
            .limit(limit.unwrap_or(100));

        posts.load(conn).await
    }

    pub async fn user(&self, conn: &mut Connection) -> QueryResult<User> {
        User::find(self.user_id, conn).await
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Clone, Default, Insertable)]
#[diesel(table_name = crate::schema::post)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPost<'a> {
    pub user_id: i32,
    pub content: &'a str,
}

impl<'a> NewPost<'a> {
    pub fn new(user_id: i32, content: &'a str) -> Self {
        NewPost { user_id, content }
    }

    pub async fn create(&self, conn: &mut Connection) -> QueryResult<Post> {
        insert_into(post::table)
            .values(self)
            .returning(post::table::all_columns())
            .get_result(conn)
            .await
    }
}

mod p {
    #![allow(dead_code)]
    #![allow(unused_variables)]

    use paste::paste;

    struct Zomg<T>(T);

    #[macro_export]
    macro_rules! zomg {
        (new-record () -> { $pub:vis $name:ident $(($field:ident: $type:ty))* }) => {
            paste! {
                $pub struct [<New $name Record>]<'a> {
                    $($field : $type),*
                }
            }
        };

        (new-record ( id : $type:ty, $($next:tt)* ) -> { $($output:tt)* }) => {
            zomg!(new-record ( $($next)* ) -> { $($output)* });
        };

        (new-record ( $field:ident : String ) -> { $($output:tt)* }) => {
            zomg!(new-record () -> { $($output)* ($field : &'a str) });
        };

        (new-record ( $field:ident : $type:ty ) -> { $($output:tt)* }) => {
            zomg!(new-record () -> { $($output)* ($field : $type) });
        };

        (new-record ( $field:ident : $type:ty, $($next:tt)* ) -> { $($output:tt)* }) => (::defile::defile! {
            zomg!(new-record ( $(@$next)* ) -> { $($output)* ($field : $type) });
        });

        (@record () -> { $(#[$attr:meta])* $pub:vis $name:ident $(($field:ident: $type:ty))* }) => {
            paste! {
                $(#[$attr])*
                $pub struct [<$name Record>] {
                    $($field : $type),*
                }
            }

            zomg!(new-record ( $($field : $type),* ) -> { $pub $name });
        };

        (@record ( $field:ident : Zomg<$type:ty>, $($next:tt)* ) -> { $($output:tt)* }) => {
            paste! {
                zomg!(@record ( $($next)* ) -> { $($output)* ([<$field _id>] : i32) });
            }
        };

        (@record ( $field:ident : $type:ty ) -> { $($output:tt)* }) => {
            zomg!(@record () -> { $($output)* ($field : $type) });
        };

        (@record ( $field:ident : $type:ty, $($next:tt)* ) -> { $($output:tt)* }) => {
            zomg!(@record ( $($next)* ) -> { $($output)* ($field : $type) });
        };

        (@model () -> { $(#[$attr:meta])* $pub:vis $name:ident $(($field:ident: $type:ty))* }) => {
            $pub struct $name {
                $($field : $type),*
            }
        };

        (@model ( $field:ident : Zomg<$type:ty>, $($next:tt)* ) -> { $($output:tt)* }) => {
            zomg!(@model ( $($next)* ) -> { $($output)* ($field : $type) });
        };

        (@model ( $field:ident : $type:ty ) -> { $($output:tt)* }) => {
            zomg!(@model () -> { $($output)* ($field : $type) });
        };

        (@model ( $field:ident : $type:ty, $($next:tt)* ) -> { $($output:tt)* }) => {
            zomg!(@model ( $($next)* ) -> { $($output)* ($field : $type) });
        };

        // Entry point zomg!(struct { .. });
        ($(#[$attr:meta])* $pub:vis struct $name:ident { $($fields:tt)* } ) => {
            zomg!(@model ( $($fields)* ) -> { $pub $name });

            zomg!(@record ( $($fields)* ) -> { $(#[$attr])* $pub $name });
        };
    }

    zomg!(
        #[derive(Debug)]
        pub struct Comment {
            id: i32,
            post: Zomg<Post>,
            content: String,
        }
    );

    use crate::app::Connection;
    use diesel::QueryResult;

    // Just a newtype, not part of macro.
    struct AvatarUrl(String);

    struct Profile {
        id: i32,
        avatar: Option<AvatarUrl>,
    }

    struct ProfileRecord {
        id: i32,
        avatar: Option<AvatarUrl>,
    }

    struct NewProfileRecord<'a> {
        avatar: Option<&'a AvatarUrl>,
    }

    struct User {
        id: i32,
        name: String,
        profile: Profile,
    }

    struct UserRecord {
        id: i32,
        name: String,
        profile_id: i32,
    }

    struct NewUserRecord<'a> {
        name: &'a str,
        profile_id: i32,
    }

    struct Post {
        id: i32,
        author: User,
        content: String,
    }

    impl Post {
        pub fn new_record(user_id: i32, content: &str) -> NewPostRecord {
            todo!()
        }

        pub fn or_new_record<'a>(user: &'a User, content: &'a str) -> NewPostRecord<'a> {
            todo!()
        }

        // @TODO this should possibly be a separate #[derive(FromRecord)]
        pub async fn from_record(record: &PostRecord, conn: &mut Connection) -> Self {
            todo!()
        }

        pub async fn from_records<'a>(
            records: impl IntoIterator<Item = &'a PostRecord>,
            conn: &'a mut Connection,
        ) -> Vec<Self> {
            todo!()
        }
    }

    struct PostRecord {
        id: i32,
        user_id: i32,
        content: String,
    }

    struct NewPostRecord<'a> {
        user_id: i32,
        content: &'a str,
    }

    impl<'a> NewPostRecord<'a> {
        pub fn new(user_id: i32, content: &str) -> Self {
            todo!()
        }

        pub async fn create(&self, conn: &mut Connection) -> QueryResult<PostRecord> {
            todo!()
        }
    }
}
