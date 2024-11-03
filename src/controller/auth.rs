use crate::{app::AuthSession, model, view};
use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_messages::Messages;
use serde::Deserialize;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn form(messages: Messages, Query(NextUrl { next }): Query<NextUrl>) -> view::Login {
    view::Login {
        messages: messages.into_iter().collect(),
        next,
    }
}

pub async fn login(
    mut session: AuthSession,
    messages: Messages,
    Form(creds): Form<model::Credentials>,
) -> impl IntoResponse {
    let user = match session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid credentials");

            let login_url = if let Some(next) = creds.next {
                format!("/login?next={}", next)
            } else {
                "/login".to_string()
            };

            return Redirect::to(&login_url).into_response();
        }
        Err(e) => {
            warn!("Error authenticating user({}): {}", creds.username, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response(); // @TODO
        }
    };

    match session.login(&user).await {
        Ok(_) => (),
        Err(e) => {
            warn!("Error logging in user({}): {}", user.username, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response(); // @TODO
        }
    }

    messages.success(format!("Successfully logged in as {}", user.username));

    if let Some(ref next) = creds.next {
        Redirect::to(next)
    } else {
        Redirect::to("/")
    }
    .into_response()
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
