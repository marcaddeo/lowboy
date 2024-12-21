use lowboy::{
    auth::{
        LoginForm, LowboyEmailVerificationView, LowboyLoginView, LowboyRegisterView,
        RegistrationForm,
    },
    model::unverified_email,
};
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

#[derive(Clone, Template, Default)]
#[template(path = "pages/auth/verify-email.html")]
pub struct EmailVerification {
    pub error: Option<String>,
    pub link: String,
}

impl LowboyEmailVerificationView for EmailVerification {
    fn set_error(self, error: unverified_email::Error) -> Self {
        Self {
            error: Some(error.to_string()),
            ..self
        }
    }

    fn set_resend_verification_link(self, link: String) -> Self {
        Self { link, ..self }
    }
}
