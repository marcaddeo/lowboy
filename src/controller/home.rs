use crate::{app::App, post::Post, user::User, view};
use axum::extract::State;

pub async fn home(State(App { database, .. }): State<App>) -> view::Home {
    let user = User::find_by_id(1, &database)
        .await
        .expect("uid 1 should exist");
    let posts = Post::find(5, &database).await.unwrap();

    view::Home { user, posts }
}
