use crate::user::User;
use fake::faker::lorem::en::Paragraph;
use fake::Dummy;

#[derive(Dummy)]
pub struct Post {
    #[dummy(expr = "User::fake()")]
    pub author: User,
    #[dummy(faker = "Paragraph(4..10)")]
    pub content: String,
}
