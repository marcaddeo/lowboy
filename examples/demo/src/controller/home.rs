use crate::model::Post;
use askama::Template;
use axum::response::IntoResponse;
use lowboy::{lowboy_view, DatabaseConnection};

#[derive(Clone, Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    pub posts: Vec<Post>,
}

pub async fn home(DatabaseConnection(mut conn): DatabaseConnection) -> impl IntoResponse {
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();

    lowboy_view!(HomeTemplate { posts }, {
        "title" => "Home",
    })
}
