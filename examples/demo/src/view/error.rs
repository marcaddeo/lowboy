use lowboy::error::LowboyErrorView;
use rinja::Template;

#[derive(Clone, Template, Default)]
#[template(path = "pages/error/index.html")]
pub struct Error {
    pub message: String,
    pub code: u16,
}

impl LowboyErrorView for Error {
    fn message(&self) -> &String {
        &self.message
    }

    fn set_message(&mut self, message: &str) -> &mut Self {
        self.message = message.to_string();
        self
    }

    fn code(&self) -> u16 {
        self.code
    }

    fn set_code(&mut self, code: u16) -> &mut Self {
        self.code = code;
        self
    }
}
