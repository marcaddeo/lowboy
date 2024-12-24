use diesel::dsl::{AsSelect, Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel::{OptionalExtension, QueryResult, Selectable};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::model::Model;
use crate::schema::permission;
use crate::Connection;

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct Permission {
    pub id: i32,
    pub name: String,
}

impl Permission {
    pub async fn find_by_name(name: &str, conn: &mut Connection) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(permission::name.eq(name))
            .first(conn)
            .await
            .optional()
    }
}

#[diesel::dsl::auto_type]
fn permission_from_clause() -> _ {
    permission::table
}

#[diesel::dsl::auto_type]
fn permission_select_clause() -> _ {
    let as_select: AsSelect<PermissionRecord, Sqlite> = PermissionRecord::as_select();
    (as_select,)
}

#[async_trait::async_trait]
impl Model for Permission {
    type RowSqlType = SqlTypeOf<Self::SelectClause>;
    type SelectClause = permission_select_clause;
    type FromClause = permission_from_clause;
    type Query = Select<Self::FromClause, Self::SelectClause>;

    fn query() -> Self::Query {
        Self::from_clause().select(Self::select_clause())
    }

    fn from_clause() -> Self::FromClause {
        permission_from_clause()
    }

    fn select_clause() -> Self::SelectClause {
        permission_select_clause()
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query()
            .filter(permission::id.eq(id))
            .first(conn)
            .await
    }
}

impl Selectable<Sqlite> for Permission {
    type SelectExpression = <Self as Model>::SelectClause;

    fn construct_selection() -> Self::SelectExpression {
        Self::select_clause()
    }
}

impl Queryable<<Permission as Model>::RowSqlType, Sqlite> for Permission {
    type Row = (PermissionRecord,);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row.0.into())
    }
}

impl From<PermissionRecord> for Permission {
    fn from(value: PermissionRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

// @note the rest of this file is to eventually be generated using lowboy_record!
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::permission)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PermissionRecord {
    pub id: i32,
    pub name: String,
}

impl PermissionRecord {
    pub fn create(name: &str) -> CreatePermissionRecord<'_> {
        CreatePermissionRecord::new(name)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<PermissionRecord> {
        permission::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdatePermissionRecord {
        UpdatePermissionRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(permission::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `Permission` model into `PermissionRecord`
impl From<Permission> for PermissionRecord {
    fn from(value: Permission) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::permission)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreatePermissionRecord<'a> {
    pub name: &'a str,
}

impl<'a> CreatePermissionRecord<'a> {
    /// Create a new `NewPermissionRecord` object
    pub fn new(name: &'a str) -> CreatePermissionRecord<'a> {
        Self { name }
    }

    /// Create a new `post` in the database
    pub async fn save(&self, conn: &mut Connection) -> QueryResult<PermissionRecord> {
        diesel::insert_into(crate::schema::permission::table)
            .values(self)
            .returning(crate::schema::permission::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::permission)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdatePermissionRecord<'a> {
    pub id: i32,
    pub name: Option<&'a str>,
}

impl<'a> UpdatePermissionRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_permission(permission: &'a Permission) -> Self {
        Self {
            id: permission.id,
            name: Some(&permission.name),
        }
    }

    pub fn from_record(record: &'a PermissionRecord) -> Self {
        Self {
            id: record.id,
            name: Some(&record.name),
        }
    }

    pub fn with_name(self, name: &'a str) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<PermissionRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::permission::all_columns)
            .get_result(conn)
            .await
    }
}

impl Permission {
    pub fn create_record(name: &str) -> CreatePermissionRecord {
        CreatePermissionRecord::new(name)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<PermissionRecord> {
        PermissionRecord::read(id, conn).await
    }

    pub fn update_record(&self) -> UpdatePermissionRecord {
        UpdatePermissionRecord::from_permission(self)
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        PermissionRecord::from(self).delete(conn).await
    }
}
