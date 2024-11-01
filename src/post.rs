use crate::id::Id;
use crate::user::User;
use fake::faker::lorem::en::Paragraph;
use fake::Dummy;
use sqlx::prelude::FromRow;

#[derive(Dummy, FromRow)]
pub struct Post {
    #[dummy(expr = "Id(None)")]
    pub id: Id,
    #[dummy(expr = "User::fake()")]
    pub author: User,
    #[dummy(faker = "Paragraph(4..10)")]
    pub content: String,
}
