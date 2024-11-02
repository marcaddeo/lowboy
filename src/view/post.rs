use crate::post::Post as PostModel;
use askama::Template;

#[derive(Template)]
#[template(path = "components/post.html")]
pub struct Post<'p> {
    pub post: &'p PostModel,
}
