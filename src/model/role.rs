use diesel::dsl::{AsSelect, Select};
use diesel::prelude::*;
use diesel::query_dsl::CompatibleType;
use diesel::sqlite::Sqlite;
use diesel::{OptionalExtension, QueryResult, Selectable};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::model::Model;
use crate::schema::{role, user_role};
use crate::Connection;

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct Role {
    pub id: i32,
    pub name: String,
}

impl Role {
    pub async fn find_by_name(name: &str, conn: &mut Connection) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(role::name.eq(name))
            .first(conn)
            .await
            .optional()
    }

    pub async fn assign(&self, user_id: i32, conn: &mut Connection) -> QueryResult<usize> {
        diesel::insert_into(user_role::table)
            .values((
                user_role::user_id.eq(user_id),
                user_role::role_id.eq(self.id),
            ))
            .execute(conn)
            .await
    }
}

#[async_trait::async_trait]
impl Model for Role {
    type Record = RoleRecord;
    type RowSqlType = (role::SqlType,);
    type Selection = (AsSelect<RoleRecord, Sqlite>,);
    type Query = Select<role::table, Self::Selection>;

    fn query() -> Self::Query {
        role::table.select((RoleRecord::as_select(),))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query().filter(role::id.eq(id)).first(conn).await
    }
}

impl CompatibleType<Role, Sqlite> for <Role as Model>::Selection {
    type SqlType = <Role as Model>::RowSqlType;
}

impl Queryable<<Role as Model>::RowSqlType, Sqlite> for Role {
    type Row = (RoleRecord,);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row.0.into())
    }
}

impl From<RoleRecord> for Role {
    fn from(value: RoleRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

// @note the rest of this file is to eventually be generated using lowboy_record!
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::role)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RoleRecord {
    pub id: i32,
    pub name: String,
}

impl RoleRecord {
    pub fn create(name: &str) -> CreateRoleRecord<'_> {
        CreateRoleRecord::new(name)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<RoleRecord> {
        role::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdateRoleRecord {
        UpdateRoleRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(role::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `Role` model into `RoleRecord`
impl From<Role> for RoleRecord {
    fn from(value: Role) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::role)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateRoleRecord<'a> {
    pub name: &'a str,
}

impl<'a> CreateRoleRecord<'a> {
    /// Create a new `NewRoleRecord` object
    pub fn new(name: &'a str) -> CreateRoleRecord<'a> {
        Self { name }
    }

    /// Create a new `post` in the database
    pub async fn save(&self, conn: &mut Connection) -> QueryResult<RoleRecord> {
        diesel::insert_into(crate::schema::role::table)
            .values(self)
            .returning(crate::schema::role::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::role)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateRoleRecord<'a> {
    pub id: i32,
    pub name: Option<&'a str>,
}

impl<'a> UpdateRoleRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_permission(permission: &'a Role) -> Self {
        Self {
            id: permission.id,
            name: Some(&permission.name),
        }
    }

    pub fn from_record(record: &'a RoleRecord) -> Self {
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

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<RoleRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::role::all_columns)
            .get_result(conn)
            .await
    }
}

impl Role {
    pub fn create_record(name: &str) -> CreateRoleRecord {
        CreateRoleRecord::new(name)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<RoleRecord> {
        RoleRecord::read(id, conn).await
    }

    pub fn update_record(&self) -> UpdateRoleRecord {
        UpdateRoleRecord::from_permission(self)
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        RoleRecord::from(self).delete(conn).await
    }
}
