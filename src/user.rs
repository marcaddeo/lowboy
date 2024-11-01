use crate::id::Id;
use anyhow::Result;
use fake::faker::company::en::CompanyName;
use fake::faker::internet::en::SafeEmail;
use fake::faker::job::en::Title;
use fake::faker::name::en::{FirstName, LastName};
use fake::Fake;
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;

#[derive(FromRow)]
pub struct User {
    pub id: Id,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub byline: String,
    pub avatar: String,
}

impl User {
    pub fn fake() -> Self {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();

        let email: String = SafeEmail().fake();

        let byline = format!(
            "{} - {}",
            Title().fake::<String>(),
            CompanyName().fake::<String>()
        );

        let avatar = format!(
            "https://avatar.iran.liara.run/username?username={}+{}",
            first_name, last_name
        );

        Self {
            id: Id(None),
            first_name,
            last_name,
            email,
            byline,
            avatar,
        }
    }

    pub async fn find_by_id(user_id: i64, db: &SqlitePool) -> Result<User> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM user WHERE id = ?", user_id)
                .fetch_one(db)
                .await?,
        )
    }
}
