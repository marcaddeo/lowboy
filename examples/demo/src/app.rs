use crate::{controller, view::Layout};
use axum::{
    routing::{get, post},
    Router,
};
use axum_login::login_required;
use diesel_async::pooled_connection::deadpool::Pool;
use lowboy::{App, AppContext, Connection, Context, Events, LowboyAuth};
use tokio_cron_scheduler::JobScheduler;

#[derive(Clone)]
pub struct DemoContext {
    pub database: Pool<Connection>,
    pub events: Events,
    pub scheduler: JobScheduler,
    #[allow(dead_code)]
    pub my_custom_thing: Vec<String>,
}

impl AppContext for DemoContext {
    fn create(
        database: Pool<Connection>,
        events: Events,
        scheduler: JobScheduler,
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
    type Layout = Layout;

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
