use crate::context::AppContext;
use axum::Router;

pub trait App<AC: AppContext>: Send {
    fn name() -> &'static str;

    fn routes() -> Router<AC>;
}
