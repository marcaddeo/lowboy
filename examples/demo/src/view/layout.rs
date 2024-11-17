use crate::model::AppUser;
use axum_messages::Message;
use lowboy::{
    model::{LowboyUserRecord, LowboyUserTrait},
    view::{LayoutContext, LowboyLayout},
};
use rinja::Template;

#[derive(Template, Default)]
#[template(path = "layout.html")]
pub struct Layout<T: LowboyUserTrait<LowboyUserRecord> + AppUser> {
    pub messages: Vec<Message>,
    pub content: String,
    pub user: Option<T>,
    pub context: LayoutContext,
}

impl<T: LowboyUserTrait<LowboyUserRecord> + Default + AppUser> LowboyLayout<T> for Layout<T> {
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

    fn set_user(&mut self, user: Option<T>) -> &mut Self {
        self.user = user;
        self
    }
}
