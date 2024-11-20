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
