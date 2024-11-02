use crate::{app::App, post::Post, user::User};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    user: User,
    posts: Vec<Post>,
}

pub async fn home(State(App { database, .. }): State<App>) -> HomeTemplate {
    let user = User::find_by_id(1, &database)
        .await
        .expect("uid 1 should exist");
    let posts = Post::find(5, &database).await.unwrap();

    HomeTemplate { user, posts }
}
