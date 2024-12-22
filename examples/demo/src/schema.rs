// @generated automatically by Diesel CLI.

// Demo App Tables.
diesel::table! {
    post (id) {
        id -> Integer,
        user_id -> Integer,
        content -> Text,
    }
}

diesel::table! {
    user_profile (id) {
        id -> Integer,
        user_id -> Integer,
        name -> Text,
        avatar -> Nullable<Text>,
        byline -> Nullable<Text>,
    }
}

diesel::joinable!(post -> user_profile (user_id));

diesel::allow_tables_to_appear_in_same_query!(user_profile, post);

// Demo App Schema & Lowboy Core Schema Interactions.
pub use lowboy::schema::email;
pub use lowboy::schema::permission;
pub use lowboy::schema::role;
pub use lowboy::schema::role_permission;
pub use lowboy::schema::user;
pub use lowboy::schema::user_role;

// Allow Demo App Schema to join with core lowboy schema.
diesel::joinable!(user_profile -> user (user_id));
diesel::joinable!(post -> user (user_id));

// Allow Demo App schema to appear in same query as core lowboy schema.
diesel::allow_tables_to_appear_in_same_query!(user_profile, email);
diesel::allow_tables_to_appear_in_same_query!(user_profile, permission);
diesel::allow_tables_to_appear_in_same_query!(user_profile, role);
diesel::allow_tables_to_appear_in_same_query!(user_profile, user_role);
diesel::allow_tables_to_appear_in_same_query!(user_profile, role_permission);
diesel::allow_tables_to_appear_in_same_query!(user_profile, user);
diesel::allow_tables_to_appear_in_same_query!(post, email);
diesel::allow_tables_to_appear_in_same_query!(post, permission);
diesel::allow_tables_to_appear_in_same_query!(post, role);
diesel::allow_tables_to_appear_in_same_query!(post, user_role);
diesel::allow_tables_to_appear_in_same_query!(post, role_permission);
diesel::allow_tables_to_appear_in_same_query!(post, user);
