use crate::controller::auth::RegistrationData;
use askama::Template;
use axum_messages::Message;

#[derive(Clone, Template)]
#[template(path = "pages/register.html")]
pub struct Register {
    pub messages: Vec<Message>,
    pub next: Option<String>,
    pub form: RegistrationData,
}
