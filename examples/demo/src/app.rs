use crate::{
    controller,
    form::RegisterForm,
    model::User,
    view::{
        self,
        auth::{Login, Register},
        Layout,
    },
};
use anyhow::Context as _;
use axum::{
    routing::{get, post},
    Router,
};
use axum_login::login_required;
use diesel_async::pooled_connection::deadpool::Pool;
use lowboy::{
    auth::{LowboyLoginForm, RegistrationDetails},
    context,
    model::LowboyUserRecord,
    App, AppContext, Connection, Context, Events, LowboyAuth,
};
use tokio_cron_scheduler::JobScheduler;

#[derive(Clone)]
pub struct DemoContext {
    pub database: Pool<Connection>,
    pub events: Events,
    pub scheduler: JobScheduler,
    #[allow(dead_code)]
    pub my_custom_thing: Vec<String>,
}

#[async_trait::async_trait]
impl AppContext for DemoContext {
    fn create(
        database: Pool<Connection>,
        events: Events,
        scheduler: JobScheduler,
    ) -> Result<Self, context::Error> {
        Ok(Self {
            database,
            events,
            scheduler,
            my_custom_thing: vec![],
        })
    }

    async fn on_new_user(
        &self,
        record: &LowboyUserRecord,
        details: RegistrationDetails,
    ) -> Result<(), context::Error> {
        let mut conn = self.database.get().await?;
        let (name, avatar) = match details {
            RegistrationDetails::Local(form) => {
                let form = form
                    .downcast_ref::<RegisterForm>()
                    .context("Couldn't downcast register form for new user creation")?;
                let (first_name, last_name) = form.name.split_once(' ').unwrap_or((&form.name, ""));
                let avatar = format!(
                    "https://avatar.iran.liara.run/username?username={}+{}",
                    first_name, last_name
                );
                (form.name.clone(), Some(avatar))
            }
            RegistrationDetails::GitHub(info) => (info.name, Some(info.avatar_url)),
            RegistrationDetails::Discord(info) => (
                info.username,
                info.avatar.map(|hash| {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png?size=256",
                        info.id, hash
                    )
                }),
            ),
        };
        User::new_record(record.id, &name)
            .with_avatar(avatar.as_deref())
            .create(&mut conn)
            .await?;
        Ok(())
    }
}

impl Context for DemoContext {
    fn database(&self) -> &Pool<Connection> {
        &self.database
    }

    fn events(&self) -> &Events {
        &self.events
    }

    fn scheduler(&self) -> &JobScheduler {
        &self.scheduler
    }
}

pub struct Demo;

impl App<DemoContext> for Demo {
    type Layout = Layout<Self::User>;
    type ErrorView = view::Error;
    type RegisterView = Register<Self::RegistrationForm>;
    type LoginView = Login<Self::LoginForm>;
    type User = User;
    type RegistrationForm = RegisterForm;
    type LoginForm = LowboyLoginForm;

    fn name() -> &'static str {
        "demo"
    }

    fn app_title() -> &'static str {
        "Demo App"
    }

    fn routes() -> Router<DemoContext> {
        Router::new()
            .route("/", get(controller::home))
            .route("/post", post(controller::post::create))
            // Previous routes require authentication.
            .route_layer(login_required!(LowboyAuth, login_url = "/login"))
    }
}

// Or, without a custom context:
//
// impl App<LowboyContext> for Demo {
//     fn name() -> &'static str {
//         "demo"
//     }
//
//     fn routes() -> Router<LowboyContext> {
//         Router::new()
//             .route("/post", post(controller::post::create))
//             .route("/", get(controller::home))
//             // Previous routes require authentication.
//             .route_layer(login_required!(LowboyAuth, login_url = "/login"))
//     }
// }
