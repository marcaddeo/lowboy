use lowboy::model::UserModel as _;
use rinja::Template;

use crate::model::{self, DemoUser, User};

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub user: User,
    pub posts: Vec<model::Post>,
}
