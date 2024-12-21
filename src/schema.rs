// @generated automatically by Diesel CLI.

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
