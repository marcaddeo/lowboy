use crate::view;
use crate::{app::DemoContext, model::Post};
use axum::extract::{Form, State};
use lowboy::AuthSession;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostCreateForm {
    message: String,
}

pub async fn create(
    auth_session: AuthSession,
    State(context): State<DemoContext>,
    Form(input): Form<PostCreateForm>,
) -> String {
    let mut conn = context.database.get().await.unwrap();
    let author = auth_session.user.expect("user should be logged in");

    let new_post = Post::new_record(author.id, &input.message);
    let record = new_post.create(&mut conn).await.unwrap();
    let post = Post::from_record(&record, &mut conn).await.unwrap();

    let oob = view::PostForm {};
    let post = view::Post { post };

    format!("{}{}", oob, post)
}