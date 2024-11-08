use std::{fmt::Display, ops::Deref};

use sqlx::prelude::{FromRow, Type};

#[derive(Clone, Debug, Default, FromRow, Type)]
#[sqlx(transparent)]
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

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(id) => write!(f, "{}", id),
            None => write!(f, "None"),
        }
    }
}
