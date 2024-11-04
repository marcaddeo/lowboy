use crate::{app::App, model, view};
use axum::extract::State;

pub async fn sun_guy(State(App { database, .. }): State<App>) -> view::SunGuy {
    let user = model::User::find_by_id(1, &database)
        .await
        .expect("uid 1 should exist");
    let posts = model::Post::find(5, &database).await.unwrap();

    let version_string = env!("VERGEN_GIT_SHA").to_string();
    view::SunGuy {
        user,
        posts,
        version_string,
    }
}
