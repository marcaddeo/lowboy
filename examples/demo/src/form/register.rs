use derive_masked::DebugMasked;
use derive_more::derive::Display;
use lowboy::auth::RegistrationForm;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub trait DemoRegistrationForm {
    fn name(&self) -> &String;
}

#[derive(Validate, Serialize, Deserialize, DebugMasked, Display, Clone, Default)]
#[display("Name: {name} Username: {username} Email: {email} Password: REDACTED Next: {next:?}")]
pub struct RegisterForm {
    #[validate(length(min = 1, message = "Your name cannot be empty"))]
    pub name: String,

    #[validate(length(
        min = 1,
        max = 32,
        message = "Username must be between 1 and 32 characters"
    ))]
    pub username: String,

    #[validate(email(message = "Email provided is not valid"))]
    pub email: String,

    #[masked]
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    password: String,

    next: Option<String>,
}

#[typetag::serde]
impl RegistrationForm for RegisterForm {
    fn empty() -> Self
    where
        Self: Sized,
    {
        <Self as Default>::default()
    }

    fn username(&self) -> &String {
        &self.username
    }

    fn email(&self) -> &String {
        &self.email
    }

    fn password(&self) -> &String {
        &self.password
    }

    fn next(&self) -> &Option<String> {
        &self.next
    }

    fn set_next(&mut self, next: Option<String>) {
        self.next = next;
    }
}

impl DemoRegistrationForm for RegisterForm {
    fn name(&self) -> &String {
        &self.name
    }
}
