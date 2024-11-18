use crate::{
    controller::auth::RegisterForm,
    model::{
        CredentialKind, Credentials, LowboyUser, LowboyUserRecord, NewLowboyUserRecord, Operation,
    },
    view::LowboyView,
    AppContext,
};
use anyhow::Result;
use async_trait::async_trait;
use axum_login::AuthnBackend;
use oauth2::{
    basic::{BasicClient, BasicRequestTokenError},
    http::header::{AUTHORIZATION, USER_AGENT},
    reqwest::{async_http_client, AsyncHttpClientError},
    url::Url,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenResponse, TokenUrl,
};
use password_auth::verify_password;
use serde::Deserialize;

pub type AuthSession = axum_login::AuthSession<LowboyAuth>;

pub trait LowboyRegisterView: LowboyView + Default + Clone {
    fn set_next(&mut self, next: Option<String>) -> &mut Self;
    fn set_form(&mut self, form: RegisterForm) -> &mut Self;
}

pub trait LowboyLoginView: LowboyView + Default + Clone {
    fn set_next(&mut self, next: Option<String>) -> &mut Self;
}

pub enum RegistrationDetails {
    GitHub(GitHubUserInfo),
    Local(RegisterForm),
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
                let user =
                    LowboyUser::find_by_username_having_password(&credentials.username, &mut conn)
                        .await?;

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
