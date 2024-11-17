use crate::schema::user;
use diesel::associations::HasTable as _;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use lowboy::model::FromRecord;
use lowboy::model::LowboyUser;
use lowboy::model::LowboyUserRecord;
use lowboy::model::LowboyUserTrait;
use lowboy::Connection;
use lowboy_record::prelude::*;

pub trait AppUser {
    fn name(&self) -> &String;
    fn avatar(&self) -> &Option<String>;
    fn byline(&self) -> &Option<String>;
}

#[apply(lowboy_record!)]
#[derive(Debug, Default, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(LowboyUserRecord, foreign_key = lowboy_user_id))]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i32,
    pub lowboy_user: Related<LowboyUser>,
    pub name: String,
    pub avatar: Option<String>,
    pub byline: Option<String>,
}

impl AppUser for User {
    fn name(&self) -> &String {
        &self.name
    }

    fn avatar(&self) -> &Option<String> {
        &self.avatar
    }

    fn byline(&self) -> &Option<String> {
        &self.byline
    }
}

impl LowboyUserTrait<LowboyUserRecord> for User {
    fn id(&self) -> i32 {
        self.id
    }

    fn username(&self) -> &String {
        &self.lowboy_user.username
    }

    fn email(&self) -> &String {
        &self.lowboy_user.email
    }

    fn password(&self) -> &Option<String> {
        &self.lowboy_user.password
    }

    fn access_token(&self) -> &Option<String> {
        &self.lowboy_user.access_token
    }
}

#[async_trait::async_trait]
impl FromRecord<LowboyUserRecord> for User {
    async fn from_record(record: &LowboyUserRecord, conn: &mut Connection) -> QueryResult<Self>
    where
        Self: Sized,
    {
        let record: UserRecord = UserRecord::table()
            .filter(user::lowboy_user_id.eq(record.id))
            .first(conn)
            .await?;
        Self::from_record(&record, conn).await
    }
}
