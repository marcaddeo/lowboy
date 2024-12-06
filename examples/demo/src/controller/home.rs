use axum::response::IntoResponse;
use lowboy::error::LowboyError;
use lowboy::extract::DatabaseConnection;
use lowboy::lowboy_view;

use crate::model::Post;
use crate::view::Home;

#[axum::debug_handler]
pub async fn home(
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, LowboyError> {
    let posts = Post::list(&mut conn, Some(5)).await?;

    Ok(lowboy_view!(Home { posts }, {
        "title" => "Home",
    }))
}
