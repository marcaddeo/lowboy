use crate::{context::AppContext, view::LowboyLayout};
use axum::Router;

pub trait App<AC: AppContext>: Send + 'static {
    fn name() -> &'static str;

    fn layout(context: &AC) -> impl LowboyLayout;

    fn routes() -> Router<AC>;
}
