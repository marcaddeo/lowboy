use crate::{
    app::{AuthSession, DatabaseConnection},
    model::{Post, User},
    view,
};

pub async fn home(
    auth_session: AuthSession,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> view::Home {
    let record = auth_session.user.expect("user should be logged in");
    let user = User::from_record(&record, &mut conn).await.unwrap();
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();

    let version_string = env!("VERGEN_GIT_SHA").to_string();
    view::Home {
        user,
        posts,
        version_string,
    }
}
