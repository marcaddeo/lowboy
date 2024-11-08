use super::Id;
use anyhow::Result;
use derive_builder::Builder;
use sqlx::{
    prelude::{FromRow, Type},
    SqlitePool,
};

#[derive(Builder, Clone, Debug, Default, FromRow, Type)]
#[builder(default, custom_constructor)]
#[builder(setter(into, strip_option))]
#[builder(build_fn(name = "fallible_build"))]
pub struct UserData {
    pub id: Id,
    pub user_id: Id,
    pub name: String,
    pub byline: Option<String>,
    pub avatar: Option<String>,
}

impl UserDataBuilder {
    pub fn build(&self) -> UserData {
        self.fallible_build()
            .expect("required fields should have been initialized")
    }
}

impl UserData {
    pub fn builder(name: impl Into<String>) -> UserDataBuilder {
        UserDataBuilder {
            name: Some(name.into()),
            ..UserDataBuilder::create_empty()
        }
    }

    pub async fn insert(user_data: &Self, db: &SqlitePool) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            r"
            INSERT INTO user_data (user_id, name, byline, avatar)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE
            SET
                name = excluded.name,
                byline = excluded.byline,
                avatar = excluded.avatar
            RETURNING *
            ",
            *user_data.user_id,
            user_data.name,
            user_data.byline,
            user_data.avatar,
        )
        .fetch_one(db)
        .await?)
    }

    pub async fn find_by_user_id(user_id: i64, db: &SqlitePool) -> Result<Self> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM user_data WHERE user_id = ?", user_id)
                .fetch_one(db)
                .await?,
        )
    }
}
