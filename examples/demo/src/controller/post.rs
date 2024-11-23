use crate::{
    app::{Demo, DemoContext},
    model::Post,
    view,
};
use axum::extract::Form;
use lowboy::extractor::{AppUser, DatabaseConnection};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostCreateForm {
    message: String,
}

pub async fn create(
    AppUser(author): AppUser<Demo, DemoContext>,
    DatabaseConnection(mut conn): DatabaseConnection,
    Form(input): Form<PostCreateForm>,
) -> String {
    let Some(author) = author else {
        return "".to_string(); // @TODO
    };

    let new_post = Post::new_record(author.id, &input.message);
    let record = new_post.create(&mut conn).await.unwrap();
    let post = Post::from_record(&record, &mut conn).await.unwrap();

    let oob = view::PostForm {};
    let post = view::Post { post };

    format!("{}{}", oob, post)
}
