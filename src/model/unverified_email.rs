use chrono::{Duration, Utc};
use diesel::dsl::{AsSelect, Eq, Filter, InnerJoin, On, Select};
use diesel::prelude::*;
use diesel::query_dsl::CompatibleType;
use diesel::sqlite::Sqlite;
use diesel::{OptionalExtension, QueryResult};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::model::{
    CreateTokenRecord, Email, EmailRecord, Model, Token, TokenRecord, UpdateEmailRecord,
};
use crate::schema::{email, token};
use crate::Connection;

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

                Ok(email_record.into())
            }
            .scope_boxed()
        })
        .await
    }
}

#[async_trait::async_trait]
impl Model for UnverifiedEmail {
    type Record = EmailRecord;
    type RowSqlType = (email::SqlType, token::SqlType);
    type Selection = (AsSelect<EmailRecord, Sqlite>, AsSelect<TokenRecord, Sqlite>);
    type Query = Select<
        Filter<
            InnerJoin<email::table, On<token::table, Eq<token::user_id, email::user_id>>>,
            Eq<email::verified, bool>,
        >,
        Self::Selection,
    >;

    fn query() -> Self::Query {
        Email::query()
            .inner_join(token::table.on(token::user_id.eq(email::user_id)))
            .filter(email::verified.eq(false))
            .select((EmailRecord::as_select(), TokenRecord::as_select()))
    }

    async fn load(id: i32, conn: &mut Connection) -> QueryResult<Self> {
        Self::query()
            .filter(email::id.eq(id))
            .filter(email::verified.eq(false))
            .first(conn)
            .await
    }
}

impl CompatibleType<UnverifiedEmail, Sqlite> for <UnverifiedEmail as Model>::Selection {
    type SqlType = <UnverifiedEmail as Model>::RowSqlType;
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
