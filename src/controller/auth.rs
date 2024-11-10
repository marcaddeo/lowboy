use crate::{
    app::AuthSession,
    model::{CredentialKind, Credentials, OAuthCredentials},
    view,
};
use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_messages::Messages;
use oauth2::CsrfToken;
use serde::Deserialize;
use tower_sessions::Session;
use tracing::warn;

pub const NEXT_URL_KEY: &str = "auth.next-url";
pub const CSRF_STATE_KEY: &str = "oauth.csrf-state";

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthzResp {
    code: String,
    state: CsrfToken,
}

pub async fn form(
    messages: Messages,
    Query(NextUrl { next }): Query<NextUrl>,
) -> impl IntoResponse {
    let version_string = env!("VERGEN_GIT_SHA").to_string();
    view::Login {
        messages: messages.into_iter().collect(),
        next,
        version_string,
    }
}

pub async fn login(
    mut auth_session: AuthSession,
    session: Session,
    messages: Messages,
    Form(input): Form<Credentials>,
) -> impl IntoResponse {
    match input.kind {
        CredentialKind::Password => {
            let credentials = input
                .password
                .as_ref()
                .expect("CredentialKind::Password password field should not be none");

            let user = match auth_session.authenticate(input.clone()).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    messages.error("Invalid credentials");

                    let login_url = if let Some(next) = input.next {
                        format!("/login?next={}", next)
                    } else {
                        "/login".to_string()
                    };

                    return Redirect::to(&login_url).into_response();
                }
                Err(e) => {
                    warn!("Error authenticating user({}): {}", credentials.username, e);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response(); // @TODO
                }
            };

            match auth_session.login(&user).await {
                Ok(_) => (),
                Err(e) => {
                    warn!("Error logging in user({}): {}", user.username, e);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response(); // @TODO
                }
            }

            messages.success(format!("Successfully logged in as {}", user.username));

            if let Some(ref next) = input.next {
                Redirect::to(next)
            } else {
                Redirect::to("/")
            }
            .into_response()
        }
        CredentialKind::OAuth => {
            let (auth_url, csrf_state) = auth_session.backend.authorize_url();

            session
                .insert(CSRF_STATE_KEY, csrf_state.secret())
                .await
                .expect("Serialization should not fail");

            session
                .insert(NEXT_URL_KEY, input.next)
                .await
                .expect("Serialization should not fail");

            Redirect::to(auth_url.as_str()).into_response()
        }
    }
}

pub async fn oauth(
    mut auth_session: AuthSession,
    messages: Messages,
    session: Session,
    Query(AuthzResp {
        code,
        state: new_state,
    }): Query<AuthzResp>,
) -> impl IntoResponse {
    let Ok(Some(old_state)) = session.get(CSRF_STATE_KEY).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let credentials = Credentials {
        kind: CredentialKind::OAuth,
        password: None,
        oauth: Some(OAuthCredentials {
            code,
            old_state,
            new_state,
        }),
        next: None,
    };

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let messages = messages.error("Invalid CSRF state");
            return (
                StatusCode::UNAUTHORIZED,
                view::Login {
                    messages: messages.into_iter().collect(),
                    next: None,
                    version_string: "".to_string(),
                },
            )
                .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Ok(Some(next)) = session.remove::<String>(NEXT_URL_KEY).await {
        Redirect::to(&next).into_response()
    } else {
        Redirect::to("/").into_response()
    }
}

pub async fn logout(mut session: AuthSession) -> impl IntoResponse {
    match session.logout().await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => {
            warn!("Error logging out user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response() // @TODO
        }
    }
}
