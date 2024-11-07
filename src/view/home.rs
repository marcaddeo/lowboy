use crate::model;
use askama::Template;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub user: model::UserWithData,
    pub posts: Vec<model::PostWithAuthor>,
    pub version_string: String,
}
