use crate::controller::auth::RegisterForm;
use askama::Template;

#[derive(Clone, Template)]
#[template(path = "pages/register.html")]
pub struct Register {
    pub next: Option<String>,
    pub form: RegisterForm,
}
