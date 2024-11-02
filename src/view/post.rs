use crate::model;
use askama::Template;

#[derive(Template)]
#[template(path = "components/post.html")]
pub struct Post<'p> {
    pub post: &'p model::Post,
}
