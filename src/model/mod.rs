use diesel::query_builder::SelectQuery;
use diesel::result::Error::{DeserializationError, NotFound};
use diesel::result::UnexpectedNullError;
use diesel::sql_types::Nullable;
use diesel::sql_types::{Integer, Text};
use diesel::{define_sql_function, QueryResult};

use crate::Connection;

mod credentials;
mod email;
mod permission;
mod role;
mod token;
pub mod unverified_email;
pub mod user;

pub use credentials::*;
pub use email::*;
pub use permission::*;
pub use role::*;
pub use token::*;
pub use unverified_email::*;
pub use user::*;

#[async_trait::async_trait]
pub trait Model {
    type RowSqlType;

    type SelectClause;
    type FromClause;
    type Query: SelectQuery;

    fn from_clause() -> Self::FromClause;

    fn select_clause() -> Self::SelectClause;

    fn query() -> Self::Query;

    // @TODO ideally i would like to be able to provide a default implementation for this, but I
    // can't quite get it working due to the generics
    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized;
}

// @note: don't really love this solution, but GROUP BY with diesel doesn't seem to be able to work
// across crates so that's problematic.
pub trait AssumeNullIsNotFoundExtension<T> {
    fn assume_null_is_not_found(self) -> QueryResult<T>;
}

impl<T> AssumeNullIsNotFoundExtension<T> for QueryResult<T> {
    fn assume_null_is_not_found(self) -> QueryResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(DeserializationError(e)) if e.is::<UnexpectedNullError>() => {
                tracing::debug!(
                    "assuming null is not found for {type_name}: {e}",
                    type_name = std::any::type_name::<T>()
                );
                Err(NotFound)
            }
            Err(e) => Err(e),
        }
    }
}

define_sql_function! {
    fn group_concat(val: Text, separator: Text) -> Text;
}

define_sql_function! {
    fn json_group_array(val: Text) -> Text;
}

// @TODO i believe Diesel will be adding general support for json_object eventually, so this should
// be a temporary solution
define_sql_function! {
    #[sql_name = "json_object"]
    fn role_record_json(a: Text, b: Integer, c: Text, d: Text) -> Text;
}

define_sql_function! {
    #[sql_name = "json_object"]
    fn permission_record_json(a: Text, b: Nullable<Integer>, c: Text, d: Nullable<Text>) -> Text;
}
