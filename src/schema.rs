// @generated automatically by Diesel CLI.

diesel::table! {
    lowboy_user (id) {
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
    user_permission (role_id, permission_id) {
        role_id -> Nullable<Integer>,
        permission_id -> Nullable<Integer>,
    }
}

diesel::table! {
    user_role (user_id, role_id) {
        user_id -> Nullable<Integer>,
        role_id -> Nullable<Integer>,
    }
}

diesel::joinable!(email -> lowboy_user (user_id));
diesel::joinable!(token -> lowboy_user (user_id));
diesel::joinable!(user_permission -> permission (permission_id));
diesel::joinable!(user_permission -> role (role_id));
diesel::joinable!(user_role -> lowboy_user (user_id));
diesel::joinable!(user_role -> role (role_id));

diesel::allow_tables_to_appear_in_same_query!(
    email,
    lowboy_user,
    permission,
    role,
    token,
    user_permission,
    user_role,
);
