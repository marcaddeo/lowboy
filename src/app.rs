use crate::{
    auth::{LowboyLoginView, LowboyRegisterView},
    context::AppContext,
    controller,
    model::{LowboyUserRecord, LowboyUserTrait},
    view::LowboyLayout,
};
use axum::Router;

#[allow(unused_variables)]
pub trait App<AC: AppContext + Clone>: Send + 'static {
    type Layout: LowboyLayout<Self::User>;
    type RegisterView: LowboyRegisterView;
    type LoginView: LowboyLoginView;
    type User: LowboyUserTrait<LowboyUserRecord>;

    fn name() -> &'static str;

    fn layout(context: &AC) -> Self::Layout {
        Self::Layout::default()
    }

    fn register_view(context: &AC) -> Self::RegisterView {
        Self::RegisterView::default()
    }

    fn login_view(context: &AC) -> Self::LoginView {
        Self::LoginView::default()
    }

    fn routes() -> Router<AC>;

    fn auth_routes<App: self::App<AC>>() -> Router<AC> {
        controller::auth::routes::<App, AC>()
    }
}
