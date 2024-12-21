use axum::response::IntoResponse;
use lowboy::error::LowboyError;
use lowboy::extract::{DatabaseConnection, EnsureAppUser};
use lowboy::lowboy_view;

use crate::app::{Demo, DemoContext};
use crate::model::Post;
use crate::view::Home;

#[axum::debug_handler]
pub async fn home(
    EnsureAppUser(user): EnsureAppUser<Demo, DemoContext>,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, LowboyError> {
    let posts = Post::list(&mut conn, Some(5)).await?;

    Ok(lowboy_view!(Home { user, posts }, {
        "title" => "Home",
    }))
}
