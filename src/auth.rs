#![allow(clippy::transmute_ptr_to_ref)]
use std::collections::HashMap;

use crate::{
    model::{
        CredentialKind, Credentials, LowboyUser, LowboyUserRecord, NewLowboyUserRecord, Operation,
    },
    view::LowboyView,
    AppContext,
};
use anyhow::Result;
use async_trait::async_trait;
use axum_login::AuthnBackend;
use derive_masked::DebugMasked;
use derive_more::derive::Display;
use dyn_clone::DynClone;
use mopa::mopafy;
use oauth2::{
    basic::{BasicClient, BasicRequestTokenError},
    http::header::{AUTHORIZATION, USER_AGENT},
    reqwest::{async_http_client, AsyncHttpClientError},
    url::Url,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use password_auth::verify_password;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub type AuthSession = axum_login::AuthSession<LowboyAuth>;

#[typetag::serde(tag = "RegistrationForm")]
pub trait RegistrationForm: Validate + Send + Sync + DynClone + mopa::Any {
    fn empty() -> Self
    where
        Self: Sized;
    fn username(&self) -> &String;
    fn email(&self) -> &String;
    fn password(&self) -> &String;
    fn next(&self) -> &Option<String>;
    fn set_next(&mut self, next: Option<String>);
}
dyn_clone::clone_trait_object!(RegistrationForm);
mopafy!(RegistrationForm);

#[derive(Validate, Serialize, Deserialize, DebugMasked, Display, Clone, Default)]
#[display("Username: {username} Email: {email} Password: REDACTED Next: {next:?}")]
pub struct LowboyRegisterForm {
    #[validate(length(
        min = 1,
        max = 32,
        message = "Username must be between 1 and 32 characters"
    ))]
    pub username: String,

    #[validate(email(message = "Email provided is not valid"))]
    pub email: String,

    #[masked]
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    password: String,

    next: Option<String>,
}

#[typetag::serde]
impl RegistrationForm for LowboyRegisterForm {
    fn empty() -> Self
    where
        Self: Sized,
    {
        <Self as Default>::default()
    }

    fn username(&self) -> &String {
        &self.username
    }

    fn email(&self) -> &String {
        &self.email
    }

    fn password(&self) -> &String {
        &self.password
    }

    fn next(&self) -> &Option<String> {
        &self.next
    }

    fn set_next(&mut self, next: Option<String>) {
        self.next = next;
    }
}

pub trait LowboyRegisterView<T: RegistrationForm + Default>: LowboyView + Clone + Default {
    fn set_form(&mut self, form: T) -> &mut Self;
}

#[typetag::serde(tag = "LoginForm")]
pub trait LoginForm: Validate + Send + Sync + DynClone + mopa::Any {
    fn empty() -> Self
    where
        Self: Sized;
    fn username(&self) -> &String;
    fn password(&self) -> &String;
    fn next(&self) -> &Option<String>;
    fn set_next(&mut self, next: Option<String>);
}
dyn_clone::clone_trait_object!(LoginForm);
mopafy!(LoginForm);

#[derive(Validate, Serialize, Deserialize, DebugMasked, Display, Clone, Default)]
#[display("Username: {username} Password: REDACTED Next: {next:?}")]
pub struct LowboyLoginForm {
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[masked]
    #[validate(length(min = 1, message = "Password is required"))]
    password: String,

    next: Option<String>,
}

#[typetag::serde]
impl LoginForm for LowboyLoginForm {
    fn empty() -> Self
    where
        Self: Sized,
    {
        <Self as Default>::default()
    }

    fn username(&self) -> &String {
        &self.username
    }

    fn password(&self) -> &String {
        &self.password
    }

    fn next(&self) -> &Option<String> {
        &self.next
    }

    fn set_next(&mut self, next: Option<String>) {
        self.next = next;
    }
}

pub trait LowboyLoginView<T: LoginForm + Default>: LowboyView + Clone + Default {
    fn set_form(&mut self, form: T) -> &mut Self;
}

pub enum RegistrationDetails {
    GitHub(GitHubUserInfo),
    Discord(DiscordUserInfo),
    Local(Box<dyn RegistrationForm>),
}

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq)]
#[serde(into = "String")]
pub enum IdentityProvider {
    GitHub,
    Discord,
}

impl TryFrom<IdentityProvider> for (AuthUrl, TokenUrl, &str, Vec<Scope>) {
    type Error = oauth2::url::ParseError;

    fn try_from(value: IdentityProvider) -> std::prelude::v1::Result<Self, Self::Error> {
        use IdentityProvider::*;
        Ok(match value {
            GitHub => (
                AuthUrl::new("https://github.com/login/oauth/authorize".to_string())?,
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string())?,
                "/login/oauth/github/callback",
                vec![],
            ),
            Discord => (
                AuthUrl::new("https://discord.com/oauth2/authorize".to_string())?,
                TokenUrl::new("https://discord.com/api/oauth2/token".to_string())?,
                "/login/oauth/discord/callback",
                vec![
                    Scope::new("identify".to_string()),
                    Scope::new("email".to_string()),
                ],
            ),
        })
    }
}

#[derive(Clone, Default)]
pub struct OAuthClientManager {
    clients: HashMap<IdentityProvider, (BasicClient, Vec<Scope>)>,
}

impl OAuthClientManager {
    pub fn with_client(
        self,
        idp: IdentityProvider,
        client_id: String,
        client_secret: String,
    ) -> Result<Self> {
        self.create_client(idp, client_id, client_secret)
    }

    pub fn with_github(self, client_id: String, client_secret: String) -> Result<Self> {
        self.create_client(IdentityProvider::GitHub, client_id, client_secret)
    }

    pub fn with_discord(self, client_id: String, client_secret: String) -> Result<Self> {
        self.create_client(IdentityProvider::Discord, client_id, client_secret)
    }

    pub fn get(&self, idp: &IdentityProvider) -> Option<&(BasicClient, Vec<Scope>)> {
        self.clients.get(idp)
    }

    fn create_client(
        mut self,
        idp: IdentityProvider,
        client_id: String,
        client_secret: String,
    ) -> Result<Self> {
        let (auth_url, token_url, redirect_url, scopes) = idp.clone().try_into()?;
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(RedirectUrl::new(format!(
            "http://localhost:3000{redirect_url}"
        ))?);

        self.clients.insert(idp, (client, scopes));
        Ok(self)
    }
}

#[derive(Clone)]
pub struct LowboyAuth {
    pub oauth: OAuthClientManager,
    pub context: Box<dyn AppContext>,
}

impl LowboyAuth {
    pub fn new(context: Box<dyn AppContext>) -> Result<Self> {
        let oauth = OAuthClientManager::default()
            .with_github(
                std::env::var("OAUTH_GITHUB_CLIENT_ID")
                    .expect("OAUTH_GITHUB_CLIENT_ID should be set"),
                std::env::var("OAUTH_GITHUB_CLIENT_SECRET")
                    .expect("OAUTH_GITHUB_CLIENT_SECRET should be set"),
            )?
            .with_discord(
                std::env::var("OAUTH_DISCORD_CLIENT_ID")
                    .expect("OAUTH_DISCORD_CLIENT_ID should be set"),
                std::env::var("OAUTH_DISCORD_CLIENT_SECRET")
                    .expect("OAUTH_DISCORD_CLIENT_SECRET should be set"),
            )?;

        Ok(Self { oauth, context })
    }

    pub fn authorize_url(&self, idp: &IdentityProvider) -> Option<(Url, CsrfToken)> {
        let (client, scopes) = self.oauth.get(idp)?;

        let mut auth_url = client.authorize_url(CsrfToken::new_random);
        for scope in scopes {
            auth_url = auth_url.add_scope(scope.to_owned());
        }

        Some(auth_url.url())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(reqwest::Error),

    #[error(transparent)]
    OAuth2(BasicRequestTokenError<AsyncHttpClientError>),

    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Deadpool(#[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>),

    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[error("{0}")]
    DiscordEmail(String),
}

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub login: String,
    pub email: String,
    pub avatar_url: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscordUserInfo {
    pub username: String,
    pub email: Option<String>,
    pub global_name: String,
    pub avatar: Option<String>,
}

#[async_trait]
impl AuthnBackend for LowboyAuth {
    type User = LowboyUserRecord;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let mut conn = self.context.database().get().await?;

        match credentials.kind {
            CredentialKind::Password => {
                let credentials = credentials
                    .password
                    .expect("CredentialKind::Password password field should not be none");
                let Some(user) =
                    LowboyUser::find_by_username_having_password(&credentials.username, &mut conn)
                        .await?
                else {
                    return Ok(None);
                };

                tokio::task::spawn_blocking(|| {
                    Ok(verify_password(
                        credentials.password,
                        user.password.as_ref().expect("checked is_none"),
                    )
                    .is_ok()
                    .then_some(user.into()))
                })
                .await?
            }
            CredentialKind::OAuth(IdentityProvider::GitHub) => {
                let credentials = credentials
                    .oauth
                    .expect("CredentialKind::OAuth oauth field should not be none");
                // Ensure the CSRF state has not been tampered with.
                if credentials.old_state.secret() != credentials.new_state.secret() {
                    return Ok(None);
                };

                let (client, _) = self.oauth.get(&IdentityProvider::GitHub).unwrap();
                // Process authorization code, expecting a token response back.
                let token_res = client
                    .exchange_code(AuthorizationCode::new(credentials.code))
                    .request_async(async_http_client)
                    .await
                    .map_err(Self::Error::OAuth2)?;

                // Use access token to request user info.
                let user_info = reqwest::Client::new()
                    .get("https://api.github.com/user")
                    .header(USER_AGENT.as_str(), "lowboy")
                    .header(
                        AUTHORIZATION.as_str(),
                        format!("Bearer {}", token_res.access_token().secret()),
                    )
                    .send()
                    .await
                    .map_err(Self::Error::Reqwest)?
                    .json::<GitHubUserInfo>()
                    .await
                    .map_err(Self::Error::Reqwest)?;

                // Persist user in our database so we can use `get_user`.
                let access_token = token_res.access_token().secret();
                let new_user = NewLowboyUserRecord {
                    username: &user_info.login,
                    email: &user_info.email,
                    password: None,
                    access_token: Some(access_token),
                };
                let (record, operation) = new_user.create_or_update(&mut conn).await?;

                if operation == Operation::Create {
                    self.context
                        .on_new_user(&record, RegistrationDetails::GitHub(user_info))
                        .await
                        .unwrap();
                }

                Ok(Some(record))
            }
            CredentialKind::OAuth(IdentityProvider::Discord) => {
                let credentials = credentials
                    .oauth
                    .expect("CredentialKind::OAuth oauth field should not be none");
                // Ensure the CSRF state has not been tampered with.
                if credentials.old_state.secret() != credentials.new_state.secret() {
                    return Ok(None);
                };

                let (client, _) = self.oauth.get(&IdentityProvider::Discord).unwrap();
                // Process authorization code, expecting a token response back.
                let token_res = client
                    .exchange_code(AuthorizationCode::new(credentials.code))
                    .request_async(async_http_client)
                    .await
                    .map_err(Self::Error::OAuth2)?;

                // Use access token to request user info.
                let user_info = reqwest::Client::new()
                    .get("https://discord.com/api/users/@me")
                    .header(USER_AGENT.as_str(), "lowboy")
                    .header(
                        AUTHORIZATION.as_str(),
                        format!("Bearer {}", token_res.access_token().secret()),
                    )
                    .send()
                    .await
                    .map_err(Self::Error::Reqwest)?
                    .json::<DiscordUserInfo>()
                    .await
                    .map_err(Self::Error::Reqwest)?;

                let Some(email) = user_info.email.clone() else {
                    return Err(Error::DiscordEmail(
                        "Your discord account must have an email associated with it.".to_string(),
                    ));
                };

                // Persist user in our database so we can use `get_user`.
                let access_token = token_res.access_token().secret();
                let new_user = NewLowboyUserRecord {
                    username: &user_info.username,
                    email: &email,
                    password: None,
                    access_token: Some(access_token),
                };
                let (record, operation) = new_user.create_or_update(&mut conn).await?;

                if operation == Operation::Create {
                    self.context
                        .on_new_user(&record, RegistrationDetails::Discord(user_info))
                        .await
                        .unwrap();
                }

                Ok(Some(record))
            }
        }
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        let mut conn = self.context.database().get().await?;
        Ok(Some(LowboyUser::find(*user_id, &mut conn).await?.into()))
    }
}
