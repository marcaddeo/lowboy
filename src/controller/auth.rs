use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use axum_messages::Messages;
use diesel::result::{DatabaseErrorKind, Error::DatabaseError};
use oauth2::CsrfToken;
use serde::Deserialize;
use tower_sessions::Session;
use tracing::warn;
use validator::{Validate, ValidationErrorsKind};

use crate::{
    app,
    auth::{
        IdentityProvider, LoginForm, LowboyLoginView as _, LowboyRegisterView, RegistrationDetails,
        RegistrationForm,
    },
    context::CloneableAppContext,
    lowboy_view,
    model::{
        CredentialKind, Credentials, NewLowboyUserRecord, OAuthCredentials, Operation,
        PasswordCredentials,
    },
    AuthSession,
};

const NEXT_URL_KEY: &str = "auth.next-url";
const CSRF_STATE_KEY: &str = "oauth.csrf-state";
const REGISTRATION_FORM_KEY: &str = "auth.registration-form";
const LOGIN_FORM_KEY: &str = "auth.login-form";

pub fn routes<App: app::App<AC>, AC: CloneableAppContext>() -> Router<AC> {
    Router::new()
        .route("/register", get(register_form::<App, AC>))
        .route("/register", post(register::<App, AC>))
        .route("/login", get(login_form::<App, AC>))
        .route("/login", post(login::<App, AC>))
        .route("/login/oauth/github", post(oauth_github::<App, AC>))
        .route("/login/oauth/github/callback", get(oauth_github_callback))
        .route("/login/oauth/discord", post(oauth_discord::<App, AC>))
        .route("/login/oauth/discord/callback", get(oauth_discord_callback))
        .route(
            "/login/oauth/discord/authorize",
            get(oauth_discord_authorize),
        )
        .route("/logout", get(logout))
}

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CallbackResp {
    code: String,
    state: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthzResp {
    code: String,
    state: CsrfToken,
}

pub async fn register_form<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    AuthSession { user, .. }: AuthSession,
    session: Session,
    Query(NextUrl { next }): Query<NextUrl>,
) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to(&next.unwrap_or("/".into())).into_response();
    }

    let mut form = session
        .remove(REGISTRATION_FORM_KEY)
        .await
        .unwrap()
        .unwrap_or(App::RegistrationForm::empty());

    form.set_next(next);

    lowboy_view!(App::register_view(&context).set_form(form).clone(), {
        "title" => "Register",
    })
    .into_response()
}

pub async fn register<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    AuthSession { user, .. }: AuthSession,
    session: Session,
    mut messages: Messages,
    Form(input): Form<App::RegistrationForm>,
) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to(&input.next().to_owned().unwrap_or("/".into())).into_response();
    }

    if let Err(validation) = input.validate() {
        for (_, info) in validation.into_errors() {
            if let ValidationErrorsKind::Field(errors) = info {
                for error in errors {
                    messages = messages.error(error.to_string());
                }
            }
        }

        session
            .insert(REGISTRATION_FORM_KEY, input.clone())
            .await
            .unwrap();
        return if let Some(next) = input.next().to_owned() {
            Redirect::to(&format!("/register?next={next}"))
        } else {
            Redirect::to("/register")
        }
        .into_response();
    };

    let mut conn = context.database().get().await.unwrap();

    let password = password_auth::generate_hash(input.password());
    let new_user =
        NewLowboyUserRecord::new(input.username(), input.email()).with_password(Some(&password));
    let res = new_user.create_or_update(&mut conn).await;

    match res {
        Ok(_) => messages.success("Registration successful! You can now log in."),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            messages.error("A user with the same username or email already exists")
        }
        Err(_) => messages.error("An unknown error occurred"),
    };

    if res.is_err() {
        session
            .insert(REGISTRATION_FORM_KEY, input.clone())
            .await
            .unwrap();
        if let Some(next) = input.next().to_owned() {
            Redirect::to(&format!("/register?next={next}"))
        } else {
            Redirect::to("/register")
        }
    } else {
        if let (user, Operation::Create) = res.unwrap() {
            context
                .on_new_user(&user, RegistrationDetails::Local(Box::new(input.clone())))
                .await
                .unwrap();
        }

        Redirect::to(&input.next().to_owned().unwrap_or("/login".into()))
    }
    .into_response()
}

pub async fn login_form<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    session: Session,
    Query(NextUrl { next }): Query<NextUrl>,
) -> impl IntoResponse {
    let mut form = session
        .remove(LOGIN_FORM_KEY)
        .await
        .unwrap()
        .unwrap_or(App::LoginForm::empty());

    form.set_next(next);

    lowboy_view!(App::login_view(&context).set_form(form).clone(), {
        "title" => "Login",
    })
}

pub async fn login<App: app::App<AC>, AC: CloneableAppContext>(
    mut auth_session: AuthSession,
    session: Session,
    mut messages: Messages,
    Form(input): Form<App::LoginForm>,
) -> impl IntoResponse {
    session.insert(LOGIN_FORM_KEY, input.clone()).await.unwrap();

    if let Err(validation) = input.validate() {
        for (_, info) in validation.into_errors() {
            if let ValidationErrorsKind::Field(errors) = info {
                for error in errors {
                    messages = messages.error(error.to_string());
                }
            }
        }
        return if let Some(next) = input.next().to_owned() {
            Redirect::to(&format!("/login?next={next}"))
        } else {
            Redirect::to("/login")
        }
        .into_response();
    }

    let creds = Credentials {
        kind: CredentialKind::Password,
        password: Some(PasswordCredentials {
            username: input.username().clone(),
            password: input.password().clone(),
        }),
        oauth: None,
        next: None,
    };

    let user = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid credentials");

            return if let Some(next) = input.next().to_owned() {
                Redirect::to(&format!("/login?next={next}"))
            } else {
                Redirect::to("/login")
            }
            .into_response();
        }
        Err(e) => {
            warn!("Error authenticating user({}): {}", input.username(), e);
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

    Redirect::to(&input.next().to_owned().unwrap_or("/".into())).into_response()
}

pub async fn oauth_github<App: app::App<AC>, AC: CloneableAppContext>(
    auth_session: AuthSession,
    session: Session,
    Form(input): Form<Credentials>,
) -> impl IntoResponse {
    let (auth_url, csrf_state) = auth_session
        .backend
        .authorize_url(&IdentityProvider::GitHub)
        .unwrap();

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

pub async fn oauth_github_callback(
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

    let Ok(Some(next)) = session.get::<Option<String>>(NEXT_URL_KEY).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let credentials = Credentials {
        kind: CredentialKind::OAuth(IdentityProvider::GitHub),
        password: None,
        oauth: Some(OAuthCredentials {
            code,
            old_state,
            new_state,
        }),
        next: next.clone(),
    };

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid CSRF state");

            return if let Some(next) = next.to_owned() {
                Redirect::to(&format!("/login?next={next}"))
            } else {
                Redirect::to("/login")
            }
            .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to(&next.to_owned().unwrap_or("/".into())).into_response()
}

pub async fn oauth_discord<App: app::App<AC>, AC: CloneableAppContext>(
    auth_session: AuthSession,
    session: Session,
    Form(input): Form<Credentials>,
) -> impl IntoResponse {
    let (auth_url, csrf_state) = auth_session
        .backend
        .authorize_url(&IdentityProvider::Discord)
        .unwrap();

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

pub async fn oauth_discord_callback(
    Query(CallbackResp { code, state }): Query<CallbackResp>,
) -> impl IntoResponse {
    let destination = format!("/login/oauth/discord/authorize?code={code}&state={state}");
    let html = format!(
        r#"
        <script type="text/javascript">
            window.location = "{destination}";
        </script>
        <noscript>
            <meta http-equiv="refresh" content="0;URL='{destination}'"/>
        </noscript>
        "#
    );

    lowboy_view!(html, {
        "title" => "Redirecting...",
    })
}

pub async fn oauth_discord_authorize(
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

    let Ok(Some(next)) = session.get::<Option<String>>(NEXT_URL_KEY).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let credentials = Credentials {
        kind: CredentialKind::OAuth(IdentityProvider::Discord),
        password: None,
        oauth: Some(OAuthCredentials {
            code,
            old_state,
            new_state,
        }),
        next: next.clone(),
    };

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid CSRF state");

            return if let Some(next) = next.to_owned() {
                Redirect::to(&format!("/login?next={next}"))
            } else {
                Redirect::to("/login")
            }
            .into_response();
        }
        Err(e) => {
            warn!("Error authenticating user with Discord: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to(&next.to_owned().unwrap_or("/".into())).into_response()
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
