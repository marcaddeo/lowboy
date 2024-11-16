use lowboy::{auth::LowboyLoginView, auth::LowboyRegisterView, controller::auth::RegisterForm};
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

#[derive(Clone, Template, Default)]
#[template(path = "pages/auth/register.html")]
pub struct Register {
    pub next: Option<String>,
    pub form: RegisterForm,
}

impl LowboyRegisterView for Register {
    fn set_next(&mut self, next: Option<String>) -> &mut Self {
        self.next = next;
        self
    }

    fn set_form(&mut self, form: RegisterForm) -> &mut Self {
        self.form = form;
        self
    }
}
