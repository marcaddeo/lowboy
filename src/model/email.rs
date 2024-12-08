use diesel::dsl::{AsSelect, Select};
use diesel::prelude::*;
use diesel::query_dsl::CompatibleType;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;

use crate::model::{LowboyUserRecord, Model};
use crate::schema::email;
use crate::Connection;

#[derive(Clone, Debug)]
pub struct Email {
    pub id: i32,
    pub user_id: i32,
    pub address: String,
    pub verified: bool,
}

#[async_trait::async_trait]
impl Model for Email {
    type Record = EmailRecord;
    type RowSqlType = (email::SqlType,);
    type Selection = (AsSelect<EmailRecord, Sqlite>,);
    type Query = Select<email::table, Self::Selection>;

    fn query() -> Self::Query {
        email::table.select((EmailRecord::as_select(),))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query()
            .filter(email::id.eq(id))
            .first::<Self>(conn)
            .await
    }
}

impl CompatibleType<Email, Sqlite> for <Email as Model>::Selection {
    type SqlType = <Email as Model>::RowSqlType;
}

impl Queryable<<Email as Model>::RowSqlType, Sqlite> for Email {
    type Row = (EmailRecord,);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (record,) = row;

        Ok(Self {
            id: record.id,
            user_id: record.user_id,
            address: record.address,
            verified: record.verified,
        })
    }
}

// @note the rest of this file is to eventually be generated using lowboy_record!
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
#[diesel(table_name = crate::schema::email)]
#[diesel(belongs_to(LowboyUserRecord, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct EmailRecord {
    pub id: i32,
    pub user_id: i32,
    pub address: String,
    pub verified: bool,
}

impl EmailRecord {
    pub fn create(user_id: i32, content: &str) -> CreateEmailRecord<'_> {
        CreateEmailRecord::new(user_id, content)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<EmailRecord> {
        email::table.find(id).get_result(conn).await
    }

    pub fn update(&self) -> UpdateEmailRecord {
        UpdateEmailRecord::from_record(self)
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(email::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `Email` model into `EmailRecord`
impl From<Email> for EmailRecord {
    fn from(value: Email) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            address: value.address,
            verified: value.verified,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::email)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateEmailRecord<'a> {
    pub user_id: i32,
    pub address: &'a str,
    pub verified: bool,
}

impl<'a> CreateEmailRecord<'a> {
    /// Create a new `NewEmailRecord` object
    pub fn new(user_id: i32, address: &'a str) -> CreateEmailRecord<'a> {
        Self {
            user_id,
            address,
            verified: false,
        }
    }

    /// Create a new `post` in the database
    pub async fn save(&self, conn: &mut Connection) -> QueryResult<EmailRecord> {
        diesel::insert_into(crate::schema::email::table)
            .values(self)
            .returning(crate::schema::email::table::all_columns())
            .get_result(conn)
            .await
    }
}

#[derive(Debug, Default, Identifiable, AsChangeset)]
#[diesel(table_name = crate::schema::email)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateEmailRecord<'a> {
    pub id: i32,
    pub address: Option<&'a str>,
    pub verified: Option<bool>,
}

impl<'a> UpdateEmailRecord<'a> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_email(email: &'a Email) -> Self {
        Self {
            id: email.id,
            address: Some(&email.address),
            verified: Some(email.verified),
        }
    }

    pub fn from_record(record: &'a EmailRecord) -> Self {
        Self {
            id: record.id,
            address: Some(&record.address),
            verified: Some(record.verified),
        }
    }

    pub fn with_address(self, address: &'a str) -> Self {
        Self {
            address: Some(address),
            ..self
        }
    }

    pub fn with_verified(self, verified: bool) -> Self {
        Self {
            verified: Some(verified),
            ..self
        }
    }

    pub async fn save(&self, conn: &mut Connection) -> QueryResult<EmailRecord> {
        diesel::update(self)
            .set(self)
            .returning(crate::schema::email::all_columns)
            .get_result(conn)
            .await
    }
}

impl Email {
    pub fn create_record(user_id: i32, content: &str) -> CreateEmailRecord {
        CreateEmailRecord::new(user_id, content)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<EmailRecord> {
        EmailRecord::read(id, conn).await
    }

    pub fn update_record(&self) -> UpdateEmailRecord {
        UpdateEmailRecord::from_email(self)
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        EmailRecord::from(self).delete(conn).await
    }
}
