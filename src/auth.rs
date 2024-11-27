#![allow(clippy::transmute_ptr_to_ref)]
use crate::{
    model::{
        CredentialKind, Credentials, LowboyUser, LowboyUserRecord, NewLowboyUserRecord, Operation,
    },
    view::LowboyView,
    AppContext,
};
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
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use password_auth::verify_password;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

pub type AuthSession = axum_login::AuthSession<LowboyAuth>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(reqwest::Error),

    #[error(transparent)]
    OAuth2(BasicRequestTokenError<AsyncHttpClientError>),

    #[error(transparent)]
    OAuth2Url(#[from] oauth2::url::ParseError),

    #[error("{0}")]
    OAuthClientManager(String),

    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Deadpool(#[from] deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>),

    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[error("{0}")]
    DiscordEmail(String),

    #[error("{0}")]
    AppError(String),

    #[error("missing {0} credential")]
    MissingCredential(&'static str),
}

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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityProviderConfig {
    pub kind: IdentityProvider,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub intermediary_redirect: bool,
    #[serde(default)]
    pub scopes: Vec<Scope>,
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
}

impl IdentityProviderConfig {
    pub fn new(
        kind: IdentityProvider,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            auth_url: auth_url.into(),
            token_url: token_url.into(),
            intermediary_redirect: false,
            scopes: vec![],
            extra_params: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum IdentityProvider {
    GitHub,
    Discord,
}

impl IdentityProvider {
    pub async fn fetch_registration_details(
        &self,
        token: &AccessToken,
    ) -> Result<RegistrationDetails> {
        use IdentityProvider::*;

        match *self {
            GitHub => {
                let details = reqwest::Client::new()
                    .get("https://api.github.com/user")
                    .header(USER_AGENT.as_str(), "lowboy")
                    .header(AUTHORIZATION.as_str(), format!("Bearer {}", token.secret()))
                    .send()
                    .await
                    .map_err(Error::Reqwest)?
                    .json::<GitHubUserInfo>()
                    .await
                    .map_err(Error::Reqwest)?;

                Ok(RegistrationDetails::GitHub(details))
            }

            Discord => {
                let details = reqwest::Client::new()
                    .get("https://discord.com/api/users/@me")
                    .header(USER_AGENT.as_str(), "lowboy")
                    .header(AUTHORIZATION.as_str(), format!("Bearer {}", token.secret()))
                    .send()
                    .await
                    .map_err(Error::Reqwest)?
                    .json::<DiscordUserInfo>()
                    .await
                    .map_err(Error::Reqwest)?;

                Ok(RegistrationDetails::Discord(details))
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct OAuthClientManager {
    clients: HashMap<IdentityProvider, (BasicClient, IdentityProviderConfig)>,
}

impl OAuthClientManager {
    pub fn get(&self, idp: &IdentityProvider) -> Option<&(BasicClient, IdentityProviderConfig)> {
        self.clients.get(idp)
    }

    pub fn insert(&mut self, config: IdentityProviderConfig) -> Result<&mut Self> {
        let provider = config.kind.clone();
        let intermediary_redirect = config.intermediary_redirect;
        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.to_string())?,
            Some(TokenUrl::new(config.token_url.to_string())?),
        )
        // @TODO
        .set_redirect_uri(RedirectUrl::new(format!(
            "http://localhost:3000/login/oauth/{provider}/callback?intermediary_redirect={intermediary_redirect}"
        ))?);

        self.clients.insert(provider, (client, config));
        Ok(self)
    }
}

#[derive(Clone)]
pub struct LowboyAuth {
    pub oauth: OAuthClientManager,
    pub context: Box<dyn AppContext>,
}

impl LowboyAuth {
    pub fn new(
        context: Box<dyn AppContext>,
        providers: Vec<IdentityProviderConfig>,
    ) -> Result<Self> {
        let mut oauth = OAuthClientManager::default();

        for provider in providers.into_iter() {
            oauth.insert(provider)?;
        }

        Ok(Self { oauth, context })
    }

    pub fn authorize_url(&self, idp: &IdentityProvider) -> Option<(Url, CsrfToken)> {
        let (client, config) = self.oauth.get(idp)?;

        let mut auth_url = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(config.scopes.clone());

        for (name, value) in &config.extra_params {
            auth_url = auth_url.add_extra_param(name, value);
        }

        Some(auth_url.url())
    }
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
    pub id: String,
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
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        let mut conn = self.context.database().get().await?;

        match credentials.kind {
            CredentialKind::Password => {
                let credentials = credentials
                    .password
                    .ok_or(Error::MissingCredential("password"))?;
                let Some(user) =
                    LowboyUser::find_by_username_having_password(&credentials.username, &mut conn)
                        .await?
                else {
                    return Ok(None);
                };

                tokio::task::spawn_blocking(|| {
                    Ok(verify_password(
                        credentials.password,
                        user.password.as_ref().expect("checked in query"),
                    )
                    .is_ok()
                    .then_some(user.into()))
                })
                .await?
            }
            CredentialKind::OAuth(provider) => {
                let credentials = credentials.oauth.ok_or(Error::MissingCredential("oauth"))?;
                // Ensure the CSRF state has not been tampered with.
                if credentials.old_state.secret() != credentials.new_state.secret() {
                    return Ok(None);
                };

                let (client, _) =
                    self.oauth
                        .get(&provider)
                        .ok_or(Error::OAuthClientManager(format!(
                            "failed to get client for provider: {provider}"
                        )))?;
                // Process authorization code, expecting a token response back.
                let token_res = client
                    .exchange_code(AuthorizationCode::new(credentials.code))
                    .request_async(async_http_client)
                    .await
                    .map_err(Self::Error::OAuth2)?;

                let token = token_res.access_token();
                let registration_details = provider.fetch_registration_details(token).await?;

                let (username, email) = match registration_details {
                    RegistrationDetails::GitHub(ref info) => (&info.login, &info.email),
                    RegistrationDetails::Discord(ref info) => {
                        let Some(email) = info.email.clone() else {
                            return Err(Error::DiscordEmail(
                                "Your discord account must have an email associated with it."
                                    .to_string(),
                            ));
                        };
                        (&info.username, &email.clone())
                    }
                    RegistrationDetails::Local(_) => unreachable!(),
                };

                // Persist user in our database so we can use `get_user`.
                let new_user = NewLowboyUserRecord {
                    username,
                    email,
                    password: None,
                    access_token: Some(token.secret()),
                };
                let (record, operation) = new_user.create_or_update(&mut conn).await?;

                if operation == Operation::Create {
                    self.context
                        .on_new_user(&record, registration_details)
                        .await
                        .map_err(|e| {
                            Error::AppError(format!(
                                "there was an error executing on_new_user: {e}"
                            ))
                        })?;
                }

                Ok(Some(record))
            }
        }
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        let mut conn = self.context.database().get().await?;
        Ok(Some(LowboyUser::find(*user_id, &mut conn).await?.into()))
    }
}
