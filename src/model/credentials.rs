use oauth2::CsrfToken;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub enum CredentialKind {
    Password,
    OAuth,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub kind: CredentialKind,
    #[serde(flatten)]
    pub password: Option<PasswordCredentials>,
    #[serde(flatten)]
    pub oauth: Option<OAuthCredentials>,
    pub next: Option<String>,
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
