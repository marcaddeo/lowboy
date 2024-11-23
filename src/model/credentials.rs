use crate::auth::IdentityProvider;
use oauth2::CsrfToken;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub enum CredentialKind {
    Password,
    #[serde(untagged)]
    OAuth(IdentityProvider),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub kind: CredentialKind,
    #[serde(flatten)]
    pub password: Option<PasswordCredentials>,
    #[serde(flatten)]
    pub oauth: Option<OAuthCredentials>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PasswordCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthCredentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
}
