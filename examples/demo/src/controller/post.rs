use axum::extract::Form;
use axum::response::IntoResponse;
use lowboy::error::LowboyError;
use lowboy::extract::{DatabaseConnection, EnsureAppUser};
use serde::Deserialize;

use crate::app::{Demo, DemoContext};
use crate::model::Post;
use crate::view;

#[derive(Debug, Deserialize)]
pub struct PostCreateForm {
    message: String,
}

pub async fn create(
    EnsureAppUser(author): EnsureAppUser<Demo, DemoContext>,
    DatabaseConnection(mut conn): DatabaseConnection,
    Form(input): Form<PostCreateForm>,
) -> Result<impl IntoResponse, LowboyError> {
    let new_post = Post::new_record(author.id, &input.message);
    let record = new_post.create(&mut conn).await?;
    let post = Post::from_record(&record, &mut conn).await?;

    let form = view::PostForm {};
    let post = view::Post { post };

    Ok(format!("{form}{post}"))
}
