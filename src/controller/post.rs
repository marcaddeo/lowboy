use crate::{app::App, model, view};
use axum::extract::Form;
use axum::extract::State;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostCreateForm {
    message: String,
}

pub async fn create(State(app): State<App>, Form(input): Form<PostCreateForm>) -> String {
    let author = model::User::find_by_id(1, &app.database)
        .await
        .expect("uid 1 should exist");

    let post = model::Post::new(author, input.message);
    let post = model::Post::insert(post, &app.database).await.unwrap();

    let oob = view::PostForm { swap_oob: true };
    let post = view::Post { post };

    format!("{}{}", oob, post)
}
