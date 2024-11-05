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
#[display("{id} {email} {name} {username:?} {password:?} {access_token:?} {byline:?} {avatar:?}")]
pub struct User {
    pub id: Id,
    pub email: String,
    pub name: String,
    pub username: Option<String>,
    #[masked]
    pub password: Option<String>,
    #[masked]
    pub access_token: Option<String>,

    pub byline: Option<String>,
    pub avatar: Option<String>,
}

impl User {
    pub fn fake() -> Self {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();
        let name: String = format!("{} {}", first_name, last_name);

        let email: String = SafeEmail().fake();

        let byline = Some(format!(
            "{} - {}",
            Title().fake::<String>(),
            CompanyName().fake::<String>()
        ));

        let avatar = Some(format!(
            "https://avatar.iran.liara.run/username?username={}+{}",
            first_name, last_name
        ));

        Self {
            id: Id(None),
            email,
            username: None,
            password: None,
            access_token: None,
            name,
            byline,
            avatar,
        }
    }

    pub async fn insert(user: &Self, db: &SqlitePool) -> Result<Self> {
        dbg!(&user);
        Ok(sqlx::query_as!(
            Self,
            r"
            INSERT INTO user (email, username, password, access_token, name, byline, avatar)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(email) DO UPDATE
            SET access_token = excluded.access_token
            RETURNING *
            ",
            user.email,
            user.username,
            user.password,
            user.access_token,
            user.name,
            user.byline,
            user.avatar
        )
        .fetch_one(db)
        .await?)
    }

    pub async fn find_by_id(user_id: i64, db: &SqlitePool) -> Result<Self> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM user WHERE id = ?", user_id)
                .fetch_one(db)
                .await?,
        )
    }

    #[allow(dead_code)]
    pub async fn find_by_username(username: &str, db: &SqlitePool) -> Result<Self> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM user WHERE username = ?", username)
                .fetch_one(db)
                .await?,
        )
    }

    pub async fn find_by_username_with_password(username: &str, db: &SqlitePool) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT * FROM user WHERE username = ? and password IS NOT NULL",
            username
        )
        .fetch_one(db)
        .await?)
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
            id: Id(None),
            email: value.email,
            name: value.name,
            username: Some(value.login),
            avatar: Some(value.avatar_url),
            ..Default::default()
        }
    }
}
