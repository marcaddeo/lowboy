use crate::model;
use askama::Template;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub user: model::User,
    pub posts: Vec<model::Post>,
}
