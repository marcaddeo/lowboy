use chrono::{Duration, Utc};
use diesel::dsl::{Select, SqlTypeOf};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel::QueryResult;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::model::{
    CreateTokenRecord, Email, EmailRecord, Model, Token, TokenRecord, UpdateEmailRecord,
};
use crate::schema::{email, token};
use crate::Connection;

use super::Role;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Couldn't find unverified email: {0}")]
    EmailNotFound(String),

    #[error("There was an error verifying the token")]
    TokenVerification,

    #[error(transparent)]
    VerificationQuery(#[from] diesel::result::Error),
}

#[derive(Clone, Debug)]
pub struct UnverifiedEmail {
    pub id: i32,
    pub user_id: i32,
    pub address: String,
    pub token: Token,
}

impl UnverifiedEmail {
    pub async fn new(user_id: i32, address: &str, conn: &mut Connection) -> QueryResult<Self> {
        let secret = &Uuid::new_v4().to_string();
        let expiration = Utc::now() + Duration::days(1);
        let token = TokenRecord::create(user_id, secret, expiration);

        Self::new_with_token(user_id, address, token, conn).await
    }

    pub async fn new_with_token<'a>(
        user_id: i32,
        address: &str,
        token: CreateTokenRecord<'a>,
        conn: &mut Connection,
    ) -> QueryResult<Self> {
        conn.transaction(|conn| {
            async move {
                let email = EmailRecord::create(user_id, address).save(conn).await?;

                Ok(Self {
                    id: email.id,
                    user_id: email.user_id,
                    address: email.address,
                    token: token.save(conn).await?.into(),
                })
            }
            .scope_boxed()
        })
        .await
    }

    // @TODO just realized the token is kind of a dangley boi here... this will just load _any_
    // token associated with the user.
    // do we need a join table between them? email_token? unverified_email?
    // Can fix this after.
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

    pub async fn verify(self, token: &str, conn: &mut Connection) -> Result<Email> {
        if !self.token.verify(token) {
            return Err(Error::TokenVerification);
        }

        conn.transaction(|conn| {
            async move {
                let email_record = UpdateEmailRecord::new(self.id)
                    .with_verified(true)
                    .save(conn)
                    .await?;

                self.token.delete_record(conn).await?;

                Role::find_by_name("unverified", conn)
                    .await?
                    .expect("unverified role should exist")
                    .unassign(email_record.user_id, conn)
                    .await?;

                Role::find_by_name("authenticated", conn)
                    .await?
                    .expect("authenticated role should exist")
                    .assign(email_record.user_id, conn)
                    .await?;

                Ok(email_record.into())
            }
            .scope_boxed()
        })
        .await
    }
}

#[diesel::dsl::auto_type]
fn unverified_email_from_clause() -> _ {
    email::table
        .inner_join(token::table.on(token::user_id.eq(email::user_id)))
        .filter(email::verified.eq(false))
}

#[diesel::dsl::auto_type]
fn unverified_email_select_clause() -> _ {
    (
        (email::id, email::user_id, email::address, email::verified),
        (token::id, token::user_id, token::secret, token::expiration),
    )
}

#[async_trait::async_trait]
impl Model for UnverifiedEmail {
    type RowSqlType = SqlTypeOf<Self::SelectClause>;
    type SelectClause = unverified_email_select_clause;
    type FromClause = unverified_email_from_clause;
    type Query = Select<Self::FromClause, Self::SelectClause>;

    // @TODO we never check token expiration
    fn query() -> Self::Query {
        Self::from_clause().select(Self::select_clause())
    }

    fn from_clause() -> Self::FromClause {
        unverified_email_from_clause()
    }

    fn select_clause() -> Self::SelectClause {
        unverified_email_select_clause()
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query()
            .filter(email::id.eq(id))
            .filter(email::verified.eq(false))
            .first(conn)
            .await
    }
}

impl Selectable<Sqlite> for UnverifiedEmail {
    type SelectExpression = <Self as Model>::SelectClause;

    fn construct_selection() -> Self::SelectExpression {
        Self::select_clause()
    }
}

impl Queryable<<UnverifiedEmail as Model>::RowSqlType, Sqlite> for UnverifiedEmail {
    type Row = (EmailRecord, TokenRecord);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let (email_record, token_record) = row;

        Ok(Self {
            id: email_record.id,
            user_id: email_record.user_id,
            address: email_record.address,
            token: token_record.into(),
        })
    }
}
