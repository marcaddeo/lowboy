#![allow(clippy::transmute_ptr_to_ref)]
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
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenResponse, TokenUrl,
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

pub trait LowboyRegisterView<T: RegistrationForm>: LowboyView + Clone {
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

pub trait LowboyLoginView<T: LoginForm>: LowboyView + Clone {
    fn set_form(&mut self, form: T) -> &mut Self;
}

pub enum RegistrationDetails {
    GitHub(GitHubUserInfo),
    Local(Box<dyn RegistrationForm>),
}

#[derive(Clone)]
pub struct LowboyAuth {
    pub oauth: BasicClient,
    pub context: Box<dyn AppContext>,
}

impl LowboyAuth {
    pub fn new(context: Box<dyn AppContext>) -> Result<Self> {
        let client_id = std::env::var("CLIENT_ID")
            .map(ClientId::new)
            .expect("CLIENT_ID should be provided.");
        let client_secret = std::env::var("CLIENT_SECRET")
            .map(ClientSecret::new)
            .expect("CLIENT_SECRET should be provided");

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())?;
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())?;
        let oauth = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));

        Ok(Self { oauth, context })
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.oauth.authorize_url(CsrfToken::new_random).url()
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
}

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub login: String,
    pub email: String,
    pub avatar_url: String,
    pub name: String,
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
            CredentialKind::OAuth => {
                let credentials = credentials
                    .oauth
                    .expect("CredentialKind::OAuth oauth field should not be none");
                // Ensure the CSRF state has not been tampered with.
                if credentials.old_state.secret() != credentials.new_state.secret() {
                    return Ok(None);
                };

                // Process authorization code, expecting a token response back.
                let token_res = self
                    .oauth
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
