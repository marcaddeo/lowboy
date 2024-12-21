use axum::extract::Form;
use axum::response::IntoResponse;
use lowboy::error::LowboyError;
use lowboy::extract::{DatabaseConnection, EnsureAppUser};
use lowboy::model::{Model as _, UserModel};
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
    if !author.is_authenticated() {
        return Err(LowboyError::Unauthorized);
    }

    let record = Post::create_record(author.id(), &input.message)
        .save(&mut conn)
        .await?;
    let post = Post::load(record.id, &mut conn).await?;

    let form = view::PostForm {};
    let post = view::Post { post };

    Ok(format!("{form}{post}"))
}
