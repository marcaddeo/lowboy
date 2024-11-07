use crate::{
    app::{AuthSession, DatabaseConnection},
    model::{Post, PostWithAuthor, UserWithData},
    view,
};

pub async fn home(
    auth_session: AuthSession,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> view::Home {
    let user = auth_session.user.expect("user should be logged in");
    let user = UserWithData::from_user(user, &mut conn).await.unwrap();
    let posts = Post::list(&mut conn, Some(5)).await.unwrap();
    let posts = PostWithAuthor::from_posts(posts, &mut conn).await.unwrap();

    let version_string = env!("VERGEN_GIT_SHA").to_string();
    view::Home {
        user,
        posts,
        version_string,
    }
}
