mod home;
mod login;
mod post;
mod post_form;
mod register;

use askama::Template;
use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
pub(crate) use home::*;
pub(crate) use login::*;
pub(crate) use post::*;
pub(crate) use post_form::*;
pub(crate) use register::*;

use crate::{
    app::{AuthSession, DatabaseConnection},
    model,
};

#[derive(Template)]
#[template(path = "layout.html")]
pub struct LayoutTemplate {
    content: String,
    version_string: String,
    user: Option<model::User>,
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
        LayoutTemplate {
            content: template.clone(),
            version_string,
            user,
        }
        .into_response()
    } else {
        response
    }
}

pub trait LowboyView {
    fn render(&self) -> String;
}

#[derive(Clone)]
pub struct View<T: LowboyView>(pub T);

#[derive(Clone)]
struct RenderedTemplate(String);

impl<T: LowboyView> View<T> {
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

impl<T: Template> LowboyView for T {
    fn render(&self) -> String {
        self.to_string()
    }
}
