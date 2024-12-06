use lowboy::auth::{LoginForm, LowboyLoginView, LowboyRegisterView, RegistrationForm};
use rinja::Template;

use crate::form::DemoRegistrationForm;

#[derive(Clone, Template, Default)]
#[template(path = "pages/auth/login.html")]
pub struct Login<T: LoginForm> {
    pub form: T,
}

impl<T: LoginForm + Clone + Default> LowboyLoginView<T> for Login<T> {
    fn set_form(&mut self, form: T) -> &mut Self {
        self.form = form;
        self
    }
}

#[derive(Clone, Template, Default)]
#[template(path = "pages/auth/register.html")]
pub struct Register<T: RegistrationForm + DemoRegistrationForm> {
    pub form: T,
}

impl<T: RegistrationForm + DemoRegistrationForm + Clone + Default> LowboyRegisterView<T>
    for Register<T>
{
    fn set_form(&mut self, form: T) -> &mut Self {
        self.form = form;
        self
    }
}
