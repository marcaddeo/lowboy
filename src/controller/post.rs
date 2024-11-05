use crate::app::AuthSession;
use crate::{app::App, model, view};
use axum::extract::Form;
use axum::extract::State;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostCreateForm {
    message: String,
}

pub async fn create(
    auth_session: AuthSession,
    State(app): State<App>,
    Form(input): Form<PostCreateForm>,
) -> String {
    let author = auth_session.user.expect("user should be logged in");

    let post = model::Post::new(author, input.message);
    let post = model::Post::insert(post, &app.database).await.unwrap();

    let oob = view::PostForm {};
    let post = view::Post { post };

    format!("{}{}", oob, post)
}
