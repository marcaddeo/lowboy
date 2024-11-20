use crate::model;
use crate::model::DemoUser as _;
use rinja::Template;

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub posts: Vec<model::Post>,
}
