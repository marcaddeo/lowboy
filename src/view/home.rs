use crate::{post::Post, user::User};
use askama::Template;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub user: User,
    pub posts: Vec<Post>,
}
