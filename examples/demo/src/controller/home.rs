use crate::model::Post;
use askama::Template;
use axum::{extract::State, response::IntoResponse};
use lowboy::{lowboy_view, LowboyContext};

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    pub posts: Vec<Post>,
}

pub async fn home(State(context): State<LowboyContext>) -> impl IntoResponse {
    let mut conn = context.database.get().await.unwrap();
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();

    lowboy_view!(HomeTemplate { posts }, {
        "title" => "Home",
    })
}
