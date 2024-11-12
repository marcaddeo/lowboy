use askama::Template;
use axum_messages::Message;

#[derive(Clone, Template)]
#[template(path = "pages/login.html")]
pub struct Login {
    pub messages: Vec<Message>,
    pub next: Option<String>,
}
