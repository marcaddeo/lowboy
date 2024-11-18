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
        lowboy_user_id -> Integer,
        name -> Text,
        avatar -> Nullable<Text>,
        byline -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(post, user,);
