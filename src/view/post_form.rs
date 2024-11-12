use askama::Template;

#[derive(Clone, Default, Template)]
#[template(path = "components/post-form.html")]
pub struct PostForm {}
