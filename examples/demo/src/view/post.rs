use crate::model;
use crate::model::DemoUser as _;
use rinja::Template;

#[derive(Clone, Template)]
#[template(path = "components/post.html")]
pub struct Post {
    pub post: model::Post,
}
