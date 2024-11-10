use crate::{app::DatabaseConnection, model::Post, view};
use axum::response::IntoResponse;

pub async fn home(DatabaseConnection(mut conn): DatabaseConnection) -> impl IntoResponse {
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();
    view::View(view::Home { posts })
}
