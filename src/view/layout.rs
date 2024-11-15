use super::LayoutContext;
use crate::model::User;
use axum_messages::Message;
use rinja::Template;

#[derive(Template)]
#[template(path = "layout.html")]
pub struct Layout {
    pub messages: Vec<Message>,
    pub content: String,
    pub version_string: String,
    pub user: Option<User>,
    pub context: LayoutContext,
}
