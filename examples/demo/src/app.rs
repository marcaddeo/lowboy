use crate::{controller, view::Layout};
use axum::{
    routing::{get, post},
    Router,
};
use axum_login::login_required;
use lowboy::{App, AppContext, Context, LowboyAuth};

#[derive(Clone)]
pub struct DemoContext {
    pub database: diesel_async::pooled_connection::deadpool::Pool<lowboy::Connection>,
    pub events: lowboy::Events,
    pub scheduler: tokio_cron_scheduler::JobScheduler,
    pub my_custom_thing: Vec<String>,
}

impl AppContext for DemoContext {
    fn create(
        database: diesel_async::pooled_connection::deadpool::Pool<lowboy::Connection>,
        events: lowboy::Events,
        scheduler: tokio_cron_scheduler::JobScheduler,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            database,
            events,
            scheduler,
            my_custom_thing: vec![],
        })
    }
}

impl Context for DemoContext {
    fn database(&self) -> &diesel_async::pooled_connection::deadpool::Pool<lowboy::Connection> {
        &self.database
    }

    fn events(&self) -> &lowboy::Events {
        &self.events
    }

    fn scheduler(&self) -> &tokio_cron_scheduler::JobScheduler {
        &self.scheduler
    }
}

pub struct Demo;

impl App<DemoContext> for Demo {
    fn name() -> &'static str {
        "demo"
    }

    fn routes() -> Router<DemoContext> {
        Router::new()
            .route("/post", post(controller::post::create))
            .route("/", get(controller::home))
            // Previous routes require authentication.
            .route_layer(login_required!(LowboyAuth, login_url = "/login"))
            .route("/register", get(controller::auth::register_form))
            .route("/register", post(controller::auth::register))
            .route("/login", get(controller::auth::form))
            .route("/login", post(controller::auth::login))
            .route("/login/oauth", get(controller::auth::oauth))
            .route("/logout", get(controller::auth::logout))
    }

    fn layout(_context: &DemoContext) -> impl lowboy::view::LowboyLayout {
        Layout::default()
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
