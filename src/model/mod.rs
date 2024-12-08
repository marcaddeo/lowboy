use diesel::associations::HasTable;
use diesel::query_builder::SelectQuery;
use diesel::QueryResult;

use crate::Connection;

mod credentials;
mod user;

pub use credentials::*;
pub use user::*;

#[async_trait::async_trait]
pub trait Model {
    type Record: HasTable;
    type RowSqlType;
    type Selection;
    type Query: SelectQuery;

    fn query() -> Self::Query;

    // @TODO ideally i would like to be able to provide a default implementation for this, but I
    // can't quite get it working due to the generics
    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized;
}
