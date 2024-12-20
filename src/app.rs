use axum::Router;
use serde::{Deserialize, Serialize};

use crate::auth::{LoginForm, LowboyLoginView, LowboyRegisterView, RegistrationForm};
use crate::context::CloneableAppContext;
use crate::controller;
use crate::error::{LowboyError, LowboyErrorView};
use crate::model::LowboyUserTrait;
use crate::view::LowboyLayout;

#[allow(unused_variables)]
pub trait App<AC: CloneableAppContext>: Send + 'static {
    type User: LowboyUserTrait;
    type Layout: LowboyLayout<Self::User>;
    type ErrorView: LowboyErrorView;
    type RegistrationForm: RegistrationForm
        + Clone
        + Default
        + Serialize
        + for<'de> Deserialize<'de>;
    type RegisterView: LowboyRegisterView<Self::RegistrationForm>;
    type LoginForm: LoginForm + Clone + Default + Serialize + for<'de> Deserialize<'de>;
    type LoginView: LowboyLoginView<Self::LoginForm>;

    fn name() -> &'static str;

    fn app_title() -> &'static str {
        Self::name()
    }

    fn layout(context: &AC) -> Self::Layout {
        Self::Layout::default()
    }

    fn register_view(context: &AC) -> Self::RegisterView {
        Self::RegisterView::default()
    }

    fn login_view(context: &AC) -> Self::LoginView {
        Self::LoginView::default()
    }

    fn error_view(context: &AC, error: &LowboyError) -> Self::ErrorView {
        Self::ErrorView::default()
    }

    fn routes() -> Router<AC>;

    fn auth_routes<App: self::App<AC>>() -> Router<AC> {
        controller::auth::routes::<App, AC>()
    }
}
