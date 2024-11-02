use super::{Id, User};
use anyhow::Result;
use fake::faker::lorem::en::Paragraph;
use fake::{Dummy, Fake, Faker};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use std::ops::Deref;

#[derive(Debug, Dummy)]
pub struct Post {
    #[allow(dead_code)]
    #[dummy(expr = "Id(None)")]
    pub id: Id,
    #[dummy(expr = "User::fake()")]
    pub author: User,
    #[dummy(faker = "Paragraph(4..10)")]
    pub content: String,
}

#[derive(FromRow)]
pub struct PostRow {
    pub id: i64,
    pub author_id: i64,
    pub content: String,
}

impl Post {
    pub fn fake() -> Self {
        Faker.fake()
    }

    pub async fn insert(post: Self, db: &SqlitePool) -> Result<Self> {
        let author_id = post
            .author
            .id
            .deref()
            .expect("post should have an associated author with an id");
        let row = sqlx::query!(
            "INSERT INTO post (author_id, content) VALUES (?, ?) RETURNING *",
            author_id,
            post.content
        )
        .fetch_one(db)
        .await?;

        Ok(Post {
            id: Id(Some(row.id)),
            ..post
        })
    }

    pub async fn find(limit: i64, db: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            PostRow,
            "SELECT * FROM post ORDER BY id DESC LIMIT ?",
            limit
        )
        .fetch_all(db)
        .await?;

        let mut posts = Vec::new();
        for row in rows {
            posts.push(Self::build_post(row, db).await?)
        }

        Ok(posts)
    }

    #[allow(dead_code)]
    pub async fn find_by_id(post_id: i64, db: &SqlitePool) -> Result<Self> {
        let row = sqlx::query_as!(PostRow, "SELECT * FROM post WHERE id = ?", post_id)
            .fetch_one(db)
            .await?;

        Self::build_post(row, db).await
    }

    async fn build_post(row: PostRow, db: &SqlitePool) -> Result<Post> {
        let author = User::find_by_id(row.author_id, db).await?;
        Ok(Post {
            id: Id(Some(row.id)),
            author,
            content: row.content,
        })
    }
}
