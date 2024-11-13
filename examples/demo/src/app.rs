use crate::controller;
use axum::{
    routing::{get, post},
    Router,
};
use axum_login::login_required;
use lowboy::{App, LowboyAuth, LowboyContext};

pub struct Demo;

impl App<LowboyContext> for Demo {
    fn name() -> &'static str {
        "demo"
    }

    fn routes() -> Router<LowboyContext> {
        Router::new()
            .route("/post", post(controller::post::create))
            .route("/", get(controller::home))
            // Previous routes require authentication.
            .route_layer(login_required!(LowboyAuth, login_url = "/login"))
    }
}
