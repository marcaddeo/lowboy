use askama::Template;
use axum_messages::Message;

#[derive(Template)]
#[template(path = "pages/register.html")]
pub struct Register {
    pub messages: Vec<Message>,
    pub next: Option<String>,
    pub version_string: String,
}
