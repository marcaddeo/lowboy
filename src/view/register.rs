use crate::controller::auth::RegistrationData;
use askama::Template;

#[derive(Clone, Template)]
#[template(path = "pages/register.html")]
pub struct Register {
    pub next: Option<String>,
    pub form: RegistrationData,
}
