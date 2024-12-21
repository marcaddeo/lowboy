// @generated automatically by Diesel CLI.
pub use lowboy::schema::email;
pub use lowboy::schema::lowboy_user;
pub use lowboy::schema::permission;
pub use lowboy::schema::role;
pub use lowboy::schema::role_permission;
pub use lowboy::schema::user_role;

diesel::table! {
    post (id) {
        id -> Integer,
        user_id -> Integer,
        content -> Text,
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

diesel::allow_tables_to_appear_in_same_query!(user, email);
diesel::allow_tables_to_appear_in_same_query!(user, permission);
diesel::allow_tables_to_appear_in_same_query!(user, role);
diesel::allow_tables_to_appear_in_same_query!(user, user_role);
diesel::allow_tables_to_appear_in_same_query!(user, role_permission);
diesel::allow_tables_to_appear_in_same_query!(post, email);
diesel::allow_tables_to_appear_in_same_query!(post, permission);
diesel::allow_tables_to_appear_in_same_query!(post, role);
diesel::allow_tables_to_appear_in_same_query!(post, user_role);
diesel::allow_tables_to_appear_in_same_query!(post, role_permission);
diesel::allow_tables_to_appear_in_same_query!(lowboy_user, user);
diesel::allow_tables_to_appear_in_same_query!(lowboy_user, post);
