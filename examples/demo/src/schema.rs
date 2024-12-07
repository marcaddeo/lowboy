// @generated automatically by Diesel CLI.
pub use lowboy::schema::lowboy_user;

diesel::table! {
    post (id) {
        id -> Integer,
        user_id -> Integer,
        content -> Text,
    }
}

diesel::table! {
    tower_sessions (id) {
        id -> Text,
        data -> Binary,
        expiry_date -> Integer,
    }
}

diesel::table! {
    user (id) {
        id -> Integer,
        lowboy_user_id -> Integer,
        name -> Text,
        avatar -> Nullable<Text>,
        byline -> Nullable<Text>,
    }
}

diesel::joinable!(post -> user (user_id));
diesel::joinable!(user -> lowboy_user (lowboy_user_id));

diesel::allow_tables_to_appear_in_same_query!(lowboy_user, post, tower_sessions, user,);
