use axum::response::IntoResponse;
use lowboy::error::LowboyError;
use lowboy::extract::{DatabaseConnection, EnsureAppUser};
use lowboy::lowboy_view;
use lowboy::model::UserModel;

use crate::app::{Demo, DemoContext};
use crate::model::Post;
use crate::view::Home;

#[axum::debug_handler]
pub async fn home(
    EnsureAppUser(user): EnsureAppUser<Demo, DemoContext>,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<impl IntoResponse, LowboyError> {
    let posts = Post::list(&mut conn, Some(5)).await?;

    let template = Home {
        show_post_form: user.is_authenticated(),
        posts,
    };

    Ok(lowboy_view!(template, {
        "title" => "Home",
    }))
}
