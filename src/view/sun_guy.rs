use crate::model;
use askama::Template;

#[derive(Template)]
#[template(path = "pages/sun-guy.html")]
pub struct SunGuy {
    pub user: model::User,
    pub posts: Vec<model::Post>,
    pub version_string: String,
}
