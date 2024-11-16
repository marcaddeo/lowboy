use crate::{context::AppContext, view::LowboyLayout};
use axum::Router;

#[allow(unused_variables)]
pub trait App<AC: AppContext>: Send + 'static {
    type Layout: LowboyLayout;

    fn name() -> &'static str;

    fn layout(context: &AC) -> Self::Layout {
        Self::Layout::default()
    }

    fn routes() -> Router<AC>;
}
