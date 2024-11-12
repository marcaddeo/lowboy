use crate::controller;
use axum::{
    routing::{get, post},
    Router,
};
use axum_login::login_required;
use lowboy::{App, Lowboy};

pub struct Demo;

impl App for Demo {
    fn name() -> &'static str {
        "demo"
    }

    fn routes() -> axum::Router<Lowboy> {
        Router::new()
            .route("/post", post(controller::post::create))
            .route("/", get(controller::home))
            // Previous routes require authentication.
            .route_layer(login_required!(Lowboy, login_url = "/login"))
    }
}
