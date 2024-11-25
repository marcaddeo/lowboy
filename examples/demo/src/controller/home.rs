use crate::{model::Post, view::Home};
use axum::response::IntoResponse;
use lowboy::{error::LowboyError, extract::DatabaseConnection, lowboy_view};

#[axum::debug_handler]
pub async fn home(
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, LowboyError> {
    let posts = Post::list(&mut conn, Some(5)).await?;

    Ok(lowboy_view!(Home { posts }, {
        "title" => "Home",
    }))
}
