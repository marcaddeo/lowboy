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
