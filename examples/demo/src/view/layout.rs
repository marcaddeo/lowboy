use axum_messages::Message;
use lowboy::{
    model::User,
    view::{LayoutContext, LowboyLayout},
};
use rinja::Template;

#[derive(Template, Default)]
#[template(path = "layout.html")]
pub struct Layout {
    pub messages: Vec<Message>,
    pub content: String,
    pub user: Option<User>,
    pub context: LayoutContext,
}

impl LowboyLayout for Layout {
    fn set_messages(&mut self, messages: Vec<Message>) -> &mut Self {
        self.messages = messages;
        self
    }

    fn set_content(&mut self, content: impl lowboy::view::LowboyView) -> &mut Self {
        self.content = content.to_string();
        self
    }

    fn set_context(&mut self, context: LayoutContext) -> &mut Self {
        self.context = context;
        self
    }

    fn set_user(&mut self, user: Option<User>) -> &mut Self {
        self.user = user;
        self
    }
}
