use diesel::dsl::{AsSelect, InnerJoin, LeftJoin, Select};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;
use lowboy::model::{Model, UserModel, UserRecord};
use lowboy::schema::{email, permission, role, role_permission, user_role};
use lowboy::Connection;

use crate::model::User;
use crate::schema::{post, user, user_profile};

#[derive(Clone, Debug)]
pub struct Post {
    pub id: i32,
    pub user: User,
    pub content: String,
}

impl Post {
    pub async fn list(conn: &mut Connection, limit: Option<i64>) -> QueryResult<Vec<Self>> {
        Post::query()
            .limit(limit.unwrap_or(100))
            .order_by(post::id.desc())
            .load(conn)
            .await
    }
}

#[async_trait::async_trait]
impl Model for Post {
    type RowSqlType = (AsSelect<PostRecord, Sqlite>, <User as Model>::RowSqlType);
    type Selection = (AsSelect<PostRecord, Sqlite>, <User as Model>::Selection);
    type Query = Select<
        InnerJoin<
            InnerJoin<post::table, user_profile::table>,
            InnerJoin<
                InnerJoin<user::table, email::table>,
                InnerJoin<
                    user_role::table,
                    LeftJoin<role::table, LeftJoin<role_permission::table, permission::table>>,
                >,
            >,
        >,
        Self::Selection,
    >;

    fn query() -> Self::Query {
        post::table
            // .with_user()
            .inner_join(user_profile::table)
            .inner_join(user::table.inner_join(email::table).inner_join(
                user_role::table.inner_join(
                    role::table.left_join(role_permission::table.left_join(permission::table)),
                ),
            ))
            // end
            .select((PostRecord::as_select(), User::construct_selection()))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query().filter(post::id.eq(id)).first(conn).await
    }
}

impl Queryable<<Post as Model>::RowSqlType, Sqlite> for Post {
    type Row = (PostRecord, User);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (post_record, user) = row;

        Ok(Self {
            id: post_record.id,
            user,
            content: post_record.content,
        })
    }
}

// @note the rest of this file is to eventually be generated using lowboy_record!
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
            user_id: value.user.id(),
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
            user_id: Some(post.user.id()),
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
