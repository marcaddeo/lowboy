use crate::model;
use askama::Template;

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub posts: Vec<model::Post>,
}
