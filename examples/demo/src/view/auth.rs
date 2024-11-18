use crate::form::DemoRegistrationForm;
use lowboy::auth::{LowboyLoginView, LowboyRegisterView, RegistrationForm};
use rinja::Template;

#[derive(Clone, Template, Default)]
#[template(path = "pages/auth/login.html")]
pub struct Login {
    pub next: Option<String>,
}

impl LowboyLoginView for Login {
    fn set_next(&mut self, next: Option<String>) -> &mut Self {
        self.next = next;
        self
    }
}

#[derive(Clone, Template)]
#[template(path = "pages/auth/register.html")]
pub struct Register<T: RegistrationForm + DemoRegistrationForm> {
    pub next: Option<String>,
    pub form: T,
}

impl<T: RegistrationForm + DemoRegistrationForm + Clone> LowboyRegisterView<T> for Register<T> {
    fn set_next(&mut self, next: Option<String>) -> &mut Self {
        self.next = next;
        self
    }

    fn set_form(&mut self, form: T) -> &mut Self {
        self.form = form;
        self
    }
}
