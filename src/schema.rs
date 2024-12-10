// @generated automatically by Diesel CLI.

diesel::table! {
    lowboy_user (id) {
        id -> Integer,
        username -> Text,
        email -> Text,
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

diesel::joinable!(email -> lowboy_user (user_id));
diesel::joinable!(token -> lowboy_user (user_id));

diesel::allow_tables_to_appear_in_same_query!(email, lowboy_user, token,);
