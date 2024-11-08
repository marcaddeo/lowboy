use super::user_data::UserData;
use super::Id;
use crate::app::GitHubUserInfo;
use anyhow::Result;
use axum_login::AuthUser;
use derive_masked::DebugMasked;
use derive_more::Display;
use fake::faker::company::en::CompanyName;
use fake::faker::internet::en::SafeEmail;
use fake::faker::job::en::Title;
use fake::faker::name::en::{FirstName, LastName};
use fake::Fake;
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;

#[derive(Clone, Display, DebugMasked, Default, FromRow)]
#[display("{id} {username} {email} {password:?} {access_token:?}")]
pub struct User {
    pub id: Id,
    pub username: String,
    pub email: String,
    #[masked]
    pub password: Option<String>,
    #[masked]
    pub access_token: Option<String>,
    pub data: UserData,
}

impl User {
    pub fn fake() -> Self {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();
        let name: String = format!("{} {}", first_name, last_name);

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

        let data = UserData::builder(name)
            .byline(byline)
            .avatar(avatar)
            .build();

        Self {
            id: Id(None),
            username: "fake".into(),
            email,
            password: None,
            access_token: None,
            data,
        }
    }

    pub async fn insert(user: &Self, db: &SqlitePool) -> Result<Self> {
        let record = sqlx::query!(
            r"
            INSERT INTO user (username, email, password, access_token)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(email) DO UPDATE
            SET access_token = excluded.access_token
            RETURNING *
            ",
            user.username,
            user.email,
            user.password,
            user.access_token,
        )
        .fetch_one(db)
        .await?;
        let mut user_data = user.data.clone();
        user_data.user_id = Id(Some(record.id));
        let data = UserData::insert(&user_data, db).await?;

        Ok(User {
            id: Id(Some(record.id)),
            username: record.username,
            email: record.email,
            password: record.password,
            access_token: record.access_token,
            data,
        })
    }

    pub async fn find_by_id(user_id: i64, db: &SqlitePool) -> Result<Self> {
        let record = sqlx::query!(r#"SELECT * FROM user WHERE user.id = ?"#, user_id)
            .fetch_one(db)
            .await?;
        let data = UserData::find_by_user_id(user_id, db).await?;

        Ok(User {
            id: Id(Some(record.id)),
            username: record.username,
            email: record.email,
            password: record.password,
            access_token: record.access_token,
            data,
        })
    }

    // #[allow(dead_code)]
    // pub async fn find_by_username(username: &str, db: &SqlitePool) -> Result<Self> {
    //     Ok(
    //         sqlx::query_as!(Self, "SELECT * FROM user WHERE username = ?", username)
    //             .fetch_one(db)
    //             .await?,
    //     )
    // }

    pub async fn find_by_username_with_password(username: &str, db: &SqlitePool) -> Result<Self> {
        let record = sqlx::query!(
            r#"SELECT * FROM user WHERE username = ? AND password IS NOT NULL"#,
            username
        )
        .fetch_one(db)
        .await?;
        let data = UserData::find_by_user_id(record.id, db).await?;

        Ok(User {
            id: Id(Some(record.id)),
            username: record.username,
            email: record.email,
            password: record.password,
            access_token: record.access_token,
            data,
        })
    }
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id.expect("authenticated user should have an id")
    }

    fn session_auth_hash(&self) -> &[u8] {
        if let Some(access_token) = &self.access_token {
            return access_token.as_bytes();
        }

        if let Some(password) = &self.password {
            return password.as_bytes();
        }

        &[]
    }
}

impl From<GitHubUserInfo> for User {
    fn from(value: GitHubUserInfo) -> Self {
        Self {
            username: value.login,
            email: value.email,
            data: UserData::builder(value.name)
                .avatar(value.avatar_url)
                .build(),
            ..Default::default()
        }
    }
}
