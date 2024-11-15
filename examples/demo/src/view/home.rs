use crate::model;
use rinja::Template;

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub posts: Vec<model::Post>,
}
