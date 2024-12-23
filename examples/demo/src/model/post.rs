use diesel::dsl::{Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;
use lowboy::model::{Model, UserModel, UserRecord};
use lowboy::Connection;

use crate::model::User;
use crate::schema::post;

use super::{user_from_clause, user_select_clause};

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

#[diesel::dsl::auto_type]
fn post_from_clause() -> _ {
    post::table.inner_join(user_from_clause())
}

#[diesel::dsl::auto_type]
fn post_select_clause() -> _ {
    (
        (post::id, post::user_id, post::content),
        user_select_clause(),
    )
}

#[async_trait::async_trait]
impl Model for Post {
    // @note changing this RowSqlType has "broken" being able to just use the User model directly in
    // the build method of Queryable. Previously it worked when this was:
    // type RowSqlType = (AsSelect<PostRecord, Sqlite>, <User as Model>::RowSqlType);
    // which User::RowSqlType would have had simimlar AsSelect<> for each record it was loading.
    // Is this because we can't use as_select() in the auto_type?
    type RowSqlType = SqlTypeOf<Self::SelectClause>;
    type SelectClause = post_select_clause;
    type FromClause = post_from_clause;
    type Query = Select<Self::FromClause, Self::SelectClause>;

    fn query() -> Self::Query {
        Self::from_clause().select(Self::select_clause())
    }

    fn from_clause() -> Self::FromClause {
        post_from_clause()
    }

    fn select_clause() -> Self::SelectClause {
        post_select_clause()
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query().filter(post::id.eq(id)).first(conn).await
    }
}

impl Selectable<Sqlite> for Post {
    type SelectExpression = <Self as Model>::SelectClause;

    fn construct_selection() -> Self::SelectExpression {
        Self::select_clause()
    }
}

impl Queryable<<Post as Model>::RowSqlType, Sqlite> for Post {
    type Row = (
        PostRecord,
        <User as Queryable<<User as Model>::RowSqlType, Sqlite>>::Row,
    );

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (post_record, row) = row;

        Ok(Self {
            id: post_record.id,
            user: User::build(row)?,
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
