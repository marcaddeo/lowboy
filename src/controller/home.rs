use crate::{
    app::{App, AuthSession},
    model, view,
};
use axum::extract::State;

pub async fn home(
    auth_session: AuthSession,
    State(App { database, .. }): State<App>,
) -> view::Home {
    let user = auth_session.user.expect("user should be logged in");
    let posts = model::Post::find(5, &database).await.unwrap_or_default();

    let version_string = env!("VERGEN_GIT_SHA").to_string();
    view::Home {
        user,
        posts,
        version_string,
    }
}
