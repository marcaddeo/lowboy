use crate::model::Post;
use crate::view;
use axum::extract::Form;
use lowboy::{AuthSession, DatabaseConnection};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostCreateForm {
    message: String,
}

pub async fn create(
    auth_session: AuthSession,
    DatabaseConnection(mut conn): DatabaseConnection,
    Form(input): Form<PostCreateForm>,
) -> String {
    let author = auth_session.user.expect("user should be logged in");

    let new_post = Post::new_record(author.id, &input.message);
    let record = new_post.create(&mut conn).await.unwrap();
    let post = Post::from_record(&record, &mut conn).await.unwrap();

    let oob = view::PostForm {};
    let post = view::Post { post };

    format!("{}{}", oob, post)
}
