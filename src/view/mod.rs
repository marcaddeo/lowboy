mod home;
mod login;
mod post;
mod post_form;
mod register;

pub(crate) use home::*;
pub(crate) use login::*;
pub(crate) use post::*;
pub(crate) use post_form::*;
pub(crate) use register::*;

use crate::{
    app::{AuthSession, DatabaseConnection},
    model,
};
use askama::Template;
use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use std::collections::BTreeMap;

#[derive(Template)]
#[template(path = "layout.html")]
pub struct LayoutTemplate {
    content: String,
    version_string: String,
    user: Option<model::User>,
    metadata: BTreeMap<String, String>,
}

pub async fn render_view(
    AuthSession { user, .. }: AuthSession,
    DatabaseConnection(mut conn): DatabaseConnection,
    response: Response,
) -> impl IntoResponse {
    if let Some(RenderedTemplate(template)) = response.extensions().get::<RenderedTemplate>() {
        let user = if let Some(record) = user {
            Some(model::User::from_record(&record, &mut conn).await.unwrap())
        } else {
            None
        };
        let version_string = env!("VERGEN_GIT_SHA").to_string();

        let mut metadata: BTreeMap<String, String> = BTreeMap::new();
        if let Some(ViewData(data)) = response.extensions().get::<ViewData>() {
            metadata.append(&mut data.clone());
        }

        LayoutTemplate {
            content: template.clone(),
            version_string,
            user,
            metadata,
        }
        .into_response()
    } else {
        response
    }
}

#[derive(Clone)]
pub struct ViewData(pub BTreeMap<String, String>);

impl ViewData {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl std::ops::DerefMut for ViewData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for ViewData {
    type Target = BTreeMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[macro_export]
macro_rules! view_data {
    ($($key:expr => $value:expr, )*) => {
        {
            let mut _data = $crate::view::ViewData::new();
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
        $crate::view::ViewWithData($template, view_data! { $($data)* })
    };
    ($template:expr, $data:expr) => {
        $crate::view::ViewWithData($template, $data)
    };
    ($template:expr) => {
        $crate::view::View($template)
    };
}

pub trait LowboyView {
    fn render(&self) -> String;
}

impl<T: ToString> LowboyView for T {
    fn render(&self) -> String {
        self.to_string()
    }
}

#[derive(Clone)]
pub struct View<T: LowboyView>(pub T);

#[derive(Clone)]
pub struct ViewWithData<T: LowboyView>(pub T, pub ViewData);

#[derive(Clone)]
struct RenderedTemplate(String);

impl<T: LowboyView> View<T> {
    fn render(&self) -> String {
        self.0.render()
    }
}

impl<T: LowboyView> ViewWithData<T> {
    fn render(&self) -> String {
        self.0.render()
    }
}

impl<T> IntoResponse for View<T>
where
    T: LowboyView + Send + Sync + Clone + 'static,
{
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::empty());
        let rendered = RenderedTemplate(self.render());
        response.extensions_mut().insert(rendered);
        response
    }
}

impl<T> IntoResponse for ViewWithData<T>
where
    T: LowboyView + Send + Sync + Clone + 'static,
{
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::empty());
        let rendered = RenderedTemplate(self.render());
        response.extensions_mut().insert(rendered);
        response.extensions_mut().insert(self.1);
        response
    }
}
