use crate::model;
use crate::model::AppUser as _;
use rinja::Template;

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub posts: Vec<model::Post>,
}
