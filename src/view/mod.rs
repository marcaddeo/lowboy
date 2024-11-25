use crate::{
    app,
    auth::AuthSession,
    context::CloneableAppContext,
    error::{ErrorWrapper, LowboyError, LowboyErrorView},
    lowboy_view,
    model::{FromRecord as _, LowboyUserRecord, LowboyUserTrait},
};
use axum::{
    body::Body,
    extract::State,
    response::{Html, IntoResponse, Response},
};
use axum_messages::{Message, Messages};
use dyn_clone::DynClone;
use std::collections::BTreeMap;

pub async fn error_page<App: app::App<AC>, AC: CloneableAppContext>(
    State(state): State<AC>,
    auth_session: Option<AuthSession>,
    messages: Option<Messages>,
    response: Response,
) -> impl IntoResponse {
    if let Some(ErrorWrapper(error)) = response.extensions().get::<ErrorWrapper>() {
        let message = match **error {
            // Internal server error details should not be displayed on the error page.
            LowboyError::Internal(_) => "Internal Server Error".to_string(),
            _ => error.to_string(),
        };

        let mut view = App::error_view(&state, error);
        view.set_code(response.status().into());
        view.set_message(&message);

        let view = lowboy_view!(view, {
            "title" => "Error",
        })
        .into_response();
        let html = render_view::<App, AC>(State(state), auth_session, messages, view)
            .await
            .into_response()
            .into_body();

        Response::builder()
            .status(response.status())
            .body(html)
            .unwrap()
    } else {
        response
    }
}

pub async fn render_view<App: app::App<AC>, AC: CloneableAppContext>(
    State(context): State<AC>,
    auth_session: Option<AuthSession>,
    messages: Option<Messages>,
    response: Response,
) -> impl IntoResponse {
    if let Some(ViewBox(view)) = response.extensions().get::<ViewBox>() {
        let mut conn = context.database().get().await.unwrap();
        let user = if let Some(AuthSession {
            user: Some(record), ..
        }) = auth_session
        {
            Some(App::User::from_record(&record, &mut conn).await.unwrap())
        } else {
            None
        };
        let mut layout_context = LayoutContext::default();

        layout_context.insert(
            "lowboy_version".to_string(),
            env!("VERGEN_GIT_SHA").to_string(),
        );
        layout_context.insert("app_title".to_string(), App::app_title().to_string());

        if let Some(LayoutContext(data)) = response.extensions().get::<LayoutContext>() {
            layout_context.append(&mut data.clone());
        }

        // @perf consider switching to .render() over .to_string()
        // @see https://rinja.readthedocs.io/en/stable/performance.html
        Html(
            App::layout(&context)
                .set_messages(
                    messages
                        .map(|messages| messages.into_iter().collect())
                        .unwrap_or_default(),
                )
                .set_content(view.to_string())
                .set_user(user)
                .set_context(layout_context)
                .to_string(),
        )
        .into_response()
    } else {
        response
    }
}

pub trait LowboyLayout<T: LowboyUserTrait<LowboyUserRecord>>: ToString + Default {
    fn set_messages(&mut self, messages: Vec<Message>) -> &mut Self;
    fn set_content(&mut self, content: impl LowboyView) -> &mut Self;
    fn set_context(&mut self, context: LayoutContext) -> &mut Self;
    fn set_user(&mut self, user: Option<T>) -> &mut Self;
}

pub trait LowboyView: ToString + DynClone + Send + Sync {}
dyn_clone::clone_trait_object!(LowboyView);

impl<T: ToString + Clone + Send + Sync> LowboyView for T {}

#[derive(Clone)]
pub struct View<T: LowboyView>(pub T);

#[derive(Clone)]
pub struct ViewBox(pub Box<dyn LowboyView>);

impl<T> IntoResponse for View<T>
where
    T: LowboyView + Send + Sync + Clone + 'static,
{
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::empty());
        response.extensions_mut().insert(ViewBox(Box::new(self.0)));
        response
    }
}

#[derive(Clone, Default)]
pub struct LayoutContext(pub BTreeMap<String, String>);

impl std::ops::DerefMut for LayoutContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for LayoutContext {
    type Target = BTreeMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct ViewWithContext<T: LowboyView>(pub T, pub LayoutContext);

impl<T> IntoResponse for ViewWithContext<T>
where
    T: LowboyView + Send + Sync + Clone + 'static,
{
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::empty());
        response.extensions_mut().insert(ViewBox(Box::new(self.0)));
        response.extensions_mut().insert(self.1);
        response
    }
}

#[macro_export]
macro_rules! view_data {
    ($($key:expr => $value:expr, )*) => {
        {
            let mut _data = $crate::view::LayoutContext::default();
        $(
            let _ = _data.insert($key.to_string(), $value.to_string());
        )*
            _data
        }
    }
}

#[macro_export(local_inner_macros)]
macro_rules! lowboy_view {
    ($template:expr , { $($data:tt)* }) => {
        $crate::view::ViewWithContext($template, view_data! { $($data)* })
    };
    ($template:expr, $data:expr) => {
        $crate::view::ViewWithContext($template, $data)
    };
    ($template:expr) => {
        $crate::view::View($template)
    };
}
