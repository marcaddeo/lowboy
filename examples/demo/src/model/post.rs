use crate::model::{User, UserRecord};
use crate::schema::{lowboy_user, post, user};
use diesel::dsl::{AsSelect, InnerJoin, Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::query_dsl::CompatibleType;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;
use lowboy::model::LowboyUser;
use lowboy::Connection;

/// A `Post` record
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
#[diesel(table_name = crate::schema::post)]
#[diesel(belongs_to(UserRecord, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PostRecord {
    pub id: i32,
    pub user_id: i32,
    pub content: String,
}

impl PostRecord {
    pub fn create(user_id: i32, content: &str) -> CreatePostRecord<'_> {
        CreatePostRecord::new(user_id, content)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<PostRecord> {
        post::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdatePostRecord {
        UpdatePostRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(post::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `Post` model into `PostRecord`
impl From<Post> for PostRecord {
    fn from(value: Post) -> Self {
        Self {
            id: value.id,
            content: value.content,
            user_id: value.user.id,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::post)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreatePostRecord<'a> {
    pub user_id: i32,
    pub content: &'a str,
}

impl<'a> CreatePostRecord<'a> {
    /// Create a new `NewPostRecord` object
    pub fn new(user_id: i32, content: &'a str) -> CreatePostRecord<'a> {
        Self { user_id, content }
    }

    /// Create a new `post` in the database
    pub async fn save(&self, conn: &mut Connection) -> QueryResult<PostRecord> {
        diesel::insert_into(crate::schema::post::table)
            .values(self)
            .returning(crate::schema::post::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::post)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdatePostRecord<'a> {
    pub id: i32,
    pub user_id: Option<i32>,
    pub content: Option<&'a str>,
}

impl<'a> UpdatePostRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_post(post: &'a Post) -> Self {
        Self {
            id: post.id,
            user_id: Some(post.user.id),
            content: Some(&post.content),
        }
    }

    pub fn from_record(record: &'a PostRecord) -> Self {
        Self {
            id: record.id,
            user_id: Some(record.user_id),
            content: Some(&record.content),
        }
    }

    pub fn with_user_id(self, user_id: i32) -> Self {
        Self {
            user_id: Some(user_id),
            ..self
        }
    }

    pub fn with_content(self, content: &'a str) -> Self {
        Self {
            content: Some(content),
            ..self
        }
    }

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<PostRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::post::all_columns)
            .get_result(conn)
            .await
    }
}

impl Post {
    pub fn create_record(user_id: i32, content: &str) -> CreatePostRecord {
        CreatePostRecord::new(user_id, content)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<PostRecord> {
        PostRecord::read(id, conn).await
    }

    pub fn update_record(&self) -> UpdatePostRecord {
        UpdatePostRecord::from_post(self)
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        PostRecord::from(self).delete(conn).await
    }
}

/// A `Post` model
#[derive(Clone, Debug)]
pub struct Post {
    pub id: i32,
    pub user: User,
    pub content: String,
}

type PostFields = (
    post::SqlType,
    user::SqlType,
    SqlTypeOf<lowboy_user::username>,
    SqlTypeOf<lowboy_user::email>,
);

type PostSelect = (
    AsSelect<PostRecord, Sqlite>,
    AsSelect<UserRecord, Sqlite>,
    SqlTypeOf<lowboy_user::username>,
    SqlTypeOf<lowboy_user::email>,
);

impl CompatibleType<Post, Sqlite> for PostSelect {
    type SqlType = PostFields;
}

impl Queryable<PostFields, Sqlite> for Post {
    type Row = (PostRecord, UserRecord, String, String);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (post_record, user_record, username, email) = row;

        Ok(Self {
            id: post_record.id,
            user: User {
                id: user_record.id,
                lowboy_user: LowboyUser {
                    username,
                    email,
                    ..Default::default()
                },
                name: user_record.name,
                avatar: user_record.avatar,
                byline: user_record.byline,
            },
            content: post_record.content,
        })
    }
}

type All = Select<
    InnerJoin<post::table, InnerJoin<user::table, lowboy_user::table>>,
    (
        AsSelect<PostRecord, Sqlite>,
        AsSelect<UserRecord, Sqlite>,
        lowboy_user::username,
        lowboy_user::email,
    ),
>;

impl Post {
    pub fn all() -> All {
        post::table
            .inner_join(user::table.inner_join(lowboy_user::table))
            .select((
                PostRecord::as_select(),
                UserRecord::as_select(),
                lowboy_user::username,
                lowboy_user::email,
            ))
    }

    pub async fn find(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Post::all()
            .filter(post::id.eq(id))
            .first::<Post>(conn)
            .await
    }

    pub async fn list(conn: &mut Connection, limit: Option<i64>) -> QueryResult<Vec<Self>> {
        Post::all()
            .limit(limit.unwrap_or(100))
            .order_by(post::id.desc())
            .load::<Post>(conn)
            .await
    }
}

#[async_trait::async_trait]
pub trait Loadable {
    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl Loadable for Post {
    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Post::find(id, conn).await
    }
}
