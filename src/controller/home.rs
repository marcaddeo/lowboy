use crate::{app::App, model, view};
use axum::extract::State;

pub async fn home(State(App { database, .. }): State<App>) -> view::Home {
    let user = model::User::find_by_id(1, &database)
        .await
        .expect("uid 1 should exist");
    let posts = model::Post::find(5, &database).await.unwrap();

    view::Home { user, posts }
}
