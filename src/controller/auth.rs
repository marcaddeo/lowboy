use anyhow::anyhow;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_messages::Messages;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use oauth2::CsrfToken;
use serde::Deserialize;
use tower_sessions::Session;
use tracing::warn;
use validator::{Validate, ValidationErrorsKind};

use crate::auth::{
    IdentityProvider, LoginForm as _, LowboyEmailVerificationView as _, LowboyLoginView as _,
    LowboyRegisterView as _, RegistrationDetails, RegistrationForm as _,
};
use crate::context::CloneableAppContext;
use crate::error::LowboyError;
use crate::extract::DatabaseConnection;
use crate::model::{
    unverified_email::Error as VerificationError, CredentialKind, Credentials, LowboyUser,
    Model as _, OAuthCredentials, Operation, PasswordCredentials, UnverifiedEmail,
};
use crate::{app, lowboy_view, AuthSession};

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
        .route("/login/oauth/:provider", post(oauth_init::<App, AC>))
        .route("/login/oauth/:provider/callback", get(oauth_callback))
        .route(
            "/login/oauth/:provider/authenticate",
            get(oauth_authenticate),
        )
        .route("/logout", get(logout))
        .route(
            "/email/:address/verify/:token",
            get(verify_email::<App, AC>),
        )
}

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CallbackResp {
    intermediary_redirect: bool,
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
) -> Result<impl IntoResponse, LowboyError> {
    if user.is_some() {
        return Ok(Redirect::to(&next.unwrap_or("/".into())).into_response());
    }

    let mut form = session
        .remove(REGISTRATION_FORM_KEY)
        .await?
        .unwrap_or(App::RegistrationForm::empty());

    form.set_next(next);

    Ok(
        lowboy_view!(App::register_view(&context).set_form(form).clone(), {
            "title" => "Register",
        })
        .into_response(),
    )
}

pub async fn register<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    AuthSession { user, .. }: AuthSession,
    session: Session,
    mut messages: Messages,
    Form(input): Form<App::RegistrationForm>,
) -> Result<impl IntoResponse, LowboyError> {
    if user.is_some() {
        return Ok(Redirect::to(&input.next().to_owned().unwrap_or("/".into())).into_response());
    }

    if let Err(validation) = input.validate() {
        for (_, info) in validation.into_errors() {
            if let ValidationErrorsKind::Field(errors) = info {
                for error in errors {
                    messages = messages.error(error.to_string());
                }
            }
        }

        session.insert(REGISTRATION_FORM_KEY, input.clone()).await?;
        return Ok(if let Some(next) = input.next().to_owned() {
            Redirect::to(&format!("/register?next={next}"))
        } else {
            Redirect::to("/register")
        }
        .into_response());
    };

    let mut conn = context.database().get().await?;

    let password = password_auth::generate_hash(input.password());
    let res = LowboyUser::create_record(input.username(), input.email())
        .with_password(&password)
        .save_or_update(&mut conn)
        .await;

    match res {
        Ok(_) => messages.success("Registration successful! You can now log in."),
        Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            messages.error("A user with the same username or email already exists")
        }
        Err(_) => messages.error("An unknown error occurred"),
    };

    Ok(if let Ok((user, Operation::Create)) = res {
        let user = LowboyUser::load(user.id, &mut conn).await?;
        context
            .on_new_user(&user, RegistrationDetails::Local(Box::new(input.clone())))
            .await?;

        Redirect::to(&input.next().to_owned().unwrap_or("/login".into()))
    } else {
        session.insert(REGISTRATION_FORM_KEY, input.clone()).await?;

        if let Some(next) = input.next().to_owned() {
            Redirect::to(&format!("/register?next={next}"))
        } else {
            Redirect::to("/register")
        }
    }
    .into_response())
}

pub async fn login_form<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    session: Session,
    Query(NextUrl { next }): Query<NextUrl>,
) -> Result<impl IntoResponse, LowboyError> {
    let mut form = session
        .remove(LOGIN_FORM_KEY)
        .await?
        .unwrap_or(App::LoginForm::empty());

    form.set_next(next);

    Ok(
        lowboy_view!(App::login_view(&context).set_form(form).clone(), {
            "title" => "Login",
        }),
    )
}

pub async fn login<App: app::App<AC>, AC: CloneableAppContext>(
    mut auth_session: AuthSession,
    session: Session,
    mut messages: Messages,
    Form(input): Form<App::LoginForm>,
) -> Result<impl IntoResponse, LowboyError> {
    session.insert(LOGIN_FORM_KEY, input.clone()).await?;

    if let Err(validation) = input.validate() {
        for (_, info) in validation.into_errors() {
            if let ValidationErrorsKind::Field(errors) = info {
                for error in errors {
                    messages = messages.error(error.to_string());
                }
            }
        }
        return Ok(if let Some(next) = input.next().to_owned() {
            Redirect::to(&format!("/login?next={next}"))
        } else {
            Redirect::to("/login")
        }
        .into_response());
    }

    let creds = Credentials {
        kind: CredentialKind::Password,
        password: Some(PasswordCredentials {
            username: input.username().clone(),
            password: input.password().clone(),
        }),
        oauth: None,
    };

    let user = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid credentials");

            return Ok(if let Some(next) = input.next().to_owned() {
                Redirect::to(&format!("/login?next={next}"))
            } else {
                Redirect::to("/login")
            }
            .into_response());
        }
        Err(e) => {
            return Err(anyhow!(
                "Error authenticating user({}): {e}",
                input.username()
            ))?;
        }
    };

    match auth_session.login(&user).await {
        Ok(_) => (),
        Err(e) => {
            return Err(anyhow!("Error logging in user({}): {e}", input.username()))?;
        }
    }

    Ok(Redirect::to(&input.next().to_owned().unwrap_or("/".into())).into_response())
}

pub async fn oauth_init<App: app::App<AC>, AC: CloneableAppContext>(
    auth_session: AuthSession,
    session: Session,
    Path(provider): Path<IdentityProvider>,
    Form(input): Form<App::LoginForm>,
) -> Result<impl IntoResponse, LowboyError> {
    let Some((auth_url, csrf_state)) = auth_session.backend.authorize_url(&provider) else {
        return Err(anyhow!(
            "Error getting ouath authorization url for provider: {provider}"
        ))?;
    };

    session.insert(CSRF_STATE_KEY, csrf_state.secret()).await?;
    session.insert(NEXT_URL_KEY, input.next()).await?;

    Ok(Redirect::to(auth_url.as_str()).into_response())
}

pub async fn oauth_callback(
    Path(provider): Path<IdentityProvider>,
    Query(CallbackResp {
        intermediary_redirect,
        code,
        state,
    }): Query<CallbackResp>,
) -> impl IntoResponse {
    let destination = format!("/login/oauth/{provider}/authenticate?code={code}&state={state}");
    if intermediary_redirect {
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
        .into_response()
    } else {
        Redirect::to(&destination).into_response()
    }
}

pub async fn oauth_authenticate(
    mut auth_session: AuthSession,
    messages: Messages,
    session: Session,
    Path(provider): Path<IdentityProvider>,
    Query(AuthzResp {
        code,
        state: new_state,
    }): Query<AuthzResp>,
) -> Result<impl IntoResponse, LowboyError> {
    let Ok(Some(old_state)) = session.get(CSRF_STATE_KEY).await else {
        return Err(LowboyError::BadRequest);
    };

    let next = session
        .get::<Option<String>>(NEXT_URL_KEY)
        .await?
        .unwrap_or(None);

    let credentials = Credentials {
        kind: CredentialKind::OAuth(provider),
        password: None,
        oauth: Some(OAuthCredentials {
            code,
            old_state,
            new_state,
        }),
    };

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            messages.error("Invalid CSRF state");

            return Ok(if let Some(next) = next.to_owned() {
                Redirect::to(&format!("/login?next={next}"))
            } else {
                Redirect::to("/login")
            }
            .into_response());
        }
        Err(e) => {
            return Err(anyhow!("Error during oauth authenticate: {e}"))?;
        }
    };

    if let Err(e) = auth_session.login(&user).await {
        return Err(anyhow!("Error during oauth login: {e}"))?;
    }

    Ok(Redirect::to(&next.to_owned().unwrap_or("/".into())).into_response())
}

pub async fn logout(mut session: AuthSession) -> Result<impl IntoResponse, LowboyError> {
    match session.logout().await {
        Ok(_) => Ok(Redirect::to("/").into_response()),
        Err(e) => Err(anyhow!("Error logging out user: {e}"))?,
    }
}

// @todo support ?next
pub async fn verify_email<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    DatabaseConnection(mut conn): DatabaseConnection,
    messages: Messages,
    Path((address, token)): Path<(String, String)>,
) -> Result<impl IntoResponse, LowboyError> {
    fn email_verification_view<App: app::App<AC>, AC: CloneableAppContext>(
        context: &AC,
        error: VerificationError,
    ) -> impl IntoResponse {
        let view = App::email_verification_view(context)
            // @TODO
            .set_resend_verification_link("im not actually a link lol".into())
            .set_error(error);

        lowboy_view!(view, {
            "title" => "Email Verification Error",
        })
        .into_response()
    }

    let Some(email) = UnverifiedEmail::find_by_address(&address, &mut conn).await? else {
        warn!("attempted to verify email which isn't found in database: {address}");
        return Ok(email_verification_view::<App, AC>(
            &context,
            VerificationError::EmailNotFound(address),
        )
        .into_response());
    };

    match email.verify(&token, &mut conn).await {
        Ok(_) => {
            messages.success("Your email address has been verified. You may now login.");
            Ok(Redirect::to("/login").into_response())
        }
        Err(error) => {
            warn!("couldn't verify email {address}: {error}");
            Ok(email_verification_view::<App, AC>(&context, error).into_response())
        }
    }
}
