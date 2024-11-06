// @generated automatically by Diesel CLI.

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
        username -> Text,
        email -> Text,
        password -> Nullable<Text>,
        access_token -> Nullable<Text>,
    }
}

diesel::table! {
    user_data (id) {
        id -> Integer,
        user_id -> Integer,
        name -> Text,
        avatar -> Nullable<Text>,
        byline -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    post,
    user,
    user_data,
);
