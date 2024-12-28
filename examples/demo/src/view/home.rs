use rinja::Template;

use crate::model::DemoUser;
use crate::model::Post;

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct Home {
    pub show_post_form: bool,
    pub posts: Vec<Post>,
}
