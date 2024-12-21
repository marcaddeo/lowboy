use chrono::{DateTime, Utc};
use constant_time_eq::constant_time_eq;
use diesel::dsl::{AsSelect, Select};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_async::RunQueryDsl;

use crate::model::{Model, UserRecord};
use crate::schema::token;
use crate::Connection;

#[derive(Clone, Debug)]
pub struct Token {
    pub id: i32,
    pub user_id: i32,
    pub secret: String,
    pub expiration: DateTime<Utc>,
}

impl Token {
    pub fn verify(&self, token: &str) -> bool {
        constant_time_eq(self.secret.as_bytes(), token.as_bytes())
    }
}

#[async_trait::async_trait]
impl Model for Token {
    type RowSqlType = Self::Selection;
    type Selection = (AsSelect<TokenRecord, Sqlite>,);
    type Query = Select<token::table, Self::Selection>;

    fn query() -> Self::Query {
        token::table.select((TokenRecord::as_select(),))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        // @TODO should this only load tokens that aren't expired?
        Self::query()
            .filter(token::id.eq(id))
            .first::<Self>(conn)
            .await
    }
}

impl Queryable<<Token as Model>::RowSqlType, Sqlite> for Token {
    type Row = (TokenRecord,);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (record,) = row;

        Ok(Self {
            id: record.id,
            user_id: record.user_id,
            secret: record.secret,
            expiration: record.expiration,
        })
    }
}

// @note the rest of this file is to eventually be generated using lowboy_record!
#[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
#[diesel(table_name = crate::schema::token)]
#[diesel(belongs_to(UserRecord, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TokenRecord {
    pub id: i32,
    pub user_id: i32,
    pub secret: String,
    pub expiration: DateTime<Utc>,
}

impl TokenRecord {
    pub fn create(user_id: i32, secret: &str, expiration: DateTime<Utc>) -> CreateTokenRecord {
        CreateTokenRecord::new(user_id, secret, expiration)
    }

    pub async fn read(id: i32, conn: &mut Connection) -> QueryResult<TokenRecord> {
        token::table.find(id).get_result(conn).await
    }

    pub async fn delete(&self, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(token::table.find(self.id))
            .execute(conn)
            .await
    }
}

/// Convert from a `Token` model into `TokenRecord`
impl From<Token> for TokenRecord {
    fn from(value: Token) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            secret: value.secret,
            expiration: value.expiration,
        }
    }
}

impl From<TokenRecord> for Token {
    fn from(value: TokenRecord) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            secret: value.secret,
            expiration: value.expiration,
        }
    }
}

#[derive(Debug, Default, Insertable)]
#[diesel(table_name = crate::schema::token)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CreateTokenRecord<'a> {
    pub user_id: i32,
    pub secret: &'a str,
    pub expiration: DateTime<Utc>,
}

impl<'a> CreateTokenRecord<'a> {
    /// Create a new `NewTokenRecord` object
    pub fn new(user_id: i32, secret: &'a str, expiration: DateTime<Utc>) -> CreateTokenRecord<'a> {
        Self {
            user_id,
            secret,
            expiration,
        }
    }

    /// Create a new `post` in the database
    pub async fn save(self, conn: &mut Connection) -> QueryResult<TokenRecord> {
        diesel::insert_into(crate::schema::token::table)
            .values(self)
            .returning(crate::schema::token::table::all_columns())
            .get_result(conn)
            .await
    }
}

impl Token {
    pub fn create_record(
        user_id: i32,
        secret: &str,
        expiration: DateTime<Utc>,
    ) -> CreateTokenRecord {
        CreateTokenRecord::new(user_id, secret, expiration)
    }

    pub async fn read_record(id: i32, conn: &mut Connection) -> QueryResult<TokenRecord> {
        TokenRecord::read(id, conn).await
    }

    pub async fn delete_record(self, conn: &mut Connection) -> QueryResult<usize> {
        TokenRecord::from(self).delete(conn).await
    }
}
