use crate::{app::DemoContext, model::Post, view::Home};
use axum::{extract::State, response::IntoResponse};
use lowboy::lowboy_view;

pub async fn home(State(context): State<DemoContext>) -> impl IntoResponse {
    let mut conn = context.database.get().await.unwrap();
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();

    lowboy_view!(Home { posts }, {
        "title" => "Home",
    })
}
