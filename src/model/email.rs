use derive_more::derive::Display;
use diesel::dsl::{AsSelect, Select};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel::{OptionalExtension, QueryResult, Selectable};
use diesel_async::RunQueryDsl;

use crate::model::{LowboyUserRecord, Model};
use crate::schema::email;
use crate::Connection;

use super::UnverifiedEmail;

#[derive(Clone, Debug, Display)]
#[display("{address}")]
pub struct Email {
    pub id: i32,
    pub user_id: i32,
    pub address: String,
    pub verified: bool,
}

impl Email {
    pub async fn find_by_user_id(user_id: i32, conn: &mut Connection) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(email::user_id.eq(user_id))
            .first(conn)
            .await
            .optional()
    }

    pub async fn find_by_address(
        address: &str,
        conn: &mut Connection,
    ) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(email::address.eq(address))
            .first(conn)
            .await
            .optional()
    }

    pub async fn find_by_address_having_verification(
        address: &str,
        verified: bool,
        conn: &mut Connection,
    ) -> QueryResult<Option<Self>> {
        Self::query()
            .filter(email::address.eq(address))
            .filter(email::verified.eq(verified))
            .first(conn)
            .await
            .optional()
    }
}

#[async_trait::async_trait]
impl Model for Email {
    type RowSqlType = Self::Selection;
    type Selection = (AsSelect<EmailRecord, Sqlite>,);
    type Query = Select<email::table, Self::Selection>;

    fn query() -> Self::Query {
        email::table.select((EmailRecord::as_select(),))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query().filter(email::id.eq(id)).first(conn).await
    }
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

impl From<EmailRecord> for Email {
    fn from(value: EmailRecord) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            address: value.address,
            verified: value.verified,
        }
    }
}

impl From<UnverifiedEmail> for Email {
    fn from(value: UnverifiedEmail) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            address: value.address,
            verified: false,
        }
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
    pub fn create(user_id: i32, address: &str) -> CreateEmailRecord<'_> {
        CreateEmailRecord::new(user_id, address)
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
}

impl<'a> CreateEmailRecord<'a> {
    /// Create a new `NewEmailRecord` object
    pub fn new(user_id: i32, address: &'a str) -> CreateEmailRecord<'a> {
        Self { user_id, address }
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
pub struct UpdateEmailRecord {
    pub id: i32,
    pub verified: Option<bool>,
}

impl UpdateEmailRecord {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn from_email(email: &Email) -> Self {
        Self {
            id: email.id,
            verified: Some(email.verified),
        }
    }

    pub fn from_record(record: &EmailRecord) -> Self {
        Self {
            id: record.id,
            verified: Some(record.verified),
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
    pub fn create_record(user_id: i32, address: &str) -> CreateEmailRecord {
        CreateEmailRecord::new(user_id, address)
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
