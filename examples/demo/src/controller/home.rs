use crate::{model::Post, view::Home};
use axum::response::IntoResponse;
use lowboy::{extractor::DatabaseConnection, lowboy_view};

#[axum::debug_handler]
pub async fn home(DatabaseConnection(mut conn): DatabaseConnection) -> impl IntoResponse {
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();

    lowboy_view!(Home { posts }, {
        "title" => "Home",
    })
}
