use crate::model;
use crate::model::AppUser as _;
use rinja::Template;

#[derive(Clone, Template)]
#[template(path = "components/post.html")]
pub struct Post {
    pub post: model::Post,
}
