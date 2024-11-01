use std::ops::Deref;

#[derive(Debug)]
pub struct Id(pub Option<i64>);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Self(Some(value))
    }
}
impl Deref for Id {
    type Target = Option<i64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
