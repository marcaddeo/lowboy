use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_messages::Messages;
use derive_masked::{DebugMasked, DisplayMasked};
use diesel::result::{DatabaseErrorKind, Error::DatabaseError};
use lowboy::{
    model::{CredentialKind, Credentials, OAuthCredentials, User},
    view::View,
    AuthSession,
};
use oauth2::CsrfToken;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tracing::warn;
use validator::Validate;

use crate::{app::DemoContext, view};

const NEXT_URL_KEY: &str = "auth.next-url";
const CSRF_STATE_KEY: &str = "oauth.csrf-state";
const REGISTRATION_FORM_KEY: &str = "auth.registration_form";

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthzResp {
    code: String,
    state: CsrfToken,
}

pub async fn form(Query(NextUrl { next }): Query<NextUrl>) -> impl IntoResponse {
    View(view::Login { next })
}

pub async fn register_form(
    AuthSession { user, .. }: AuthSession,
    session: Session,
) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to("/").into_response();
    }

    let form: RegisterForm = session
        .remove(REGISTRATION_FORM_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    View(view::Register { next: None, form }).into_response()
}

// @TODO figure out how to put this validation just on the NewModelRecords
#[derive(Clone, DebugMasked, Deserialize, DisplayMasked, Validate, Default, Serialize)]
pub struct RegisterForm {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1, max = 32))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    password: String,
}

pub async fn register(
    State(context): State<DemoContext>,
    AuthSession { user, .. }: AuthSession,
    session: Session,
    mut messages: Messages,
    Form(input): Form<RegisterForm>,
) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to("/").into_response();
    }

    if let Err(validation) = input.validate() {
        // @TODO just put these error messages in teh validation?
        for (field, _) in validation.into_errors() {
            let message = match field {
                "name" => "Your name cannot be empty",
                "username" => "Username must be between 1 and 32 characters",
                "email" => "Email provided is not valid",
                "password" => "Password must be at least 8 characters",
                _ => "An unknown error occurred",
            };
            messages = messages.error(message);
        }

        session.insert(REGISTRATION_FORM_KEY, input).await.unwrap();
        return Redirect::to("/register").into_response();
    };

    let mut conn = context.database.get().await.unwrap();

    let (first_name, last_name) = input.name.split_once(' ').unwrap_or((&input.name, ""));
    let avatar = format!(
        "https://avatar.iran.liara.run/username?username={}+{}",
        first_name, last_name
    );
    let password = password_auth::generate_hash(&input.password);
    let new_user = User::new_record(&input.username, &input.email).with_password(Some(&password));
    let user = new_user
        .create_or_update(&input.name, None, Some(&avatar), &mut conn)
        .await;

    match user {
        Ok(_) => messages.success("Registration successful! You can now log in."),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            messages.error("A user with the same username or email already exists")
        }
        Err(_) => messages.error("An unknown error occurred"),
    };

    if user.is_err() {
        session.insert(REGISTRATION_FORM_KEY, input).await.unwrap();
        Redirect::to("/register")
    } else {
        Redirect::to("/login")
    }
    .into_response()
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
            messages.error("Invalid CSRF state");
            return (StatusCode::UNAUTHORIZED, view::Login { next: None }).into_response();
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
