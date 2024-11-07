use crate::app::{AuthSession, DatabaseConnection};
use crate::model::{NewPost, PostWithAuthor};
use crate::view;
use axum::extract::Form;
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

    let new_post = NewPost::new(author.id, &input.message);
    let post = new_post.create(&mut conn).await.unwrap();
    let post = PostWithAuthor::from_post(post, &mut conn).await.unwrap();

    let oob = view::PostForm {};
    let post = view::Post { post };

    format!("{}{}", oob, post)
}
