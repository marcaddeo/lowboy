// @generated automatically by Diesel CLI.

// Lowboy Tables.
diesel::table! {
    user (id) {
        id -> Integer,
        username -> Text,
        password -> Nullable<Text>,
        access_token -> Nullable<Text>,
    }
}

diesel::table! {
    email (id) {
        id -> Integer,
        user_id -> Integer,
        address -> Text,
        verified -> Bool,
    }
}

diesel::table! {
    token (id) {
        id -> Integer,
        user_id -> Integer,
        secret -> Text,
        expiration -> TimestamptzSqlite,
    }
}

diesel::table! {
    permission (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    role (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    role_permission (role_id, permission_id) {
        role_id -> Integer,
        permission_id -> Integer,
    }
}

diesel::table! {
    user_role (user_id, role_id) {
        user_id -> Integer,
        role_id -> Integer,
    }
}

diesel::joinable!(email -> user (user_id));
diesel::joinable!(token -> user (user_id));
diesel::joinable!(role_permission -> permission (permission_id));
diesel::joinable!(role_permission -> role (role_id));
diesel::joinable!(user_role -> user (user_id));
diesel::joinable!(user_role -> role (role_id));

diesel::allow_tables_to_appear_in_same_query!(
    email,
    user,
    permission,
    role,
    role_permission,
    token,
    user_role,
);

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

diesel::joinable!(post -> user (user_id));
diesel::joinable!(post -> user_profile (user_id));
diesel::joinable!(user_profile -> user (user_id));

diesel::allow_tables_to_appear_in_same_query!(user, user_profile, post);

// Demo App Schema & Lowboy Core Schema Interactions.
pub use lowboy::schema::email as lowboy_email;
pub use lowboy::schema::permission as lowboy_permission;
pub use lowboy::schema::role as lowboy_role;
pub use lowboy::schema::role_permission as lowboy_role_permission;
pub use lowboy::schema::user as lowboy_user;
pub use lowboy::schema::user_role as lowboy_user_role;

// Allow Demo App Schema to join with core lowboy schema.
diesel::joinable!(user_profile -> lowboy_user (user_id));
diesel::joinable!(post -> lowboy_user (user_id));

// Allow Demo App schema to appear in same query as core lowboy schema.
diesel::allow_tables_to_appear_in_same_query!(user_profile, lowboy_email);
diesel::allow_tables_to_appear_in_same_query!(user_profile, lowboy_permission);
diesel::allow_tables_to_appear_in_same_query!(user_profile, lowboy_role);
diesel::allow_tables_to_appear_in_same_query!(user_profile, lowboy_user_role);
diesel::allow_tables_to_appear_in_same_query!(user_profile, lowboy_role_permission);
diesel::allow_tables_to_appear_in_same_query!(user_profile, lowboy_user);
diesel::allow_tables_to_appear_in_same_query!(post, lowboy_email);
diesel::allow_tables_to_appear_in_same_query!(post, lowboy_permission);
diesel::allow_tables_to_appear_in_same_query!(post, lowboy_role);
diesel::allow_tables_to_appear_in_same_query!(post, lowboy_user_role);
diesel::allow_tables_to_appear_in_same_query!(post, lowboy_role_permission);
diesel::allow_tables_to_appear_in_same_query!(post, lowboy_user);
