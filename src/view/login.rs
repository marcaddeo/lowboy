use askama::Template;

#[derive(Clone, Template)]
#[template(path = "pages/login.html")]
pub struct Login {
    pub next: Option<String>,
}
