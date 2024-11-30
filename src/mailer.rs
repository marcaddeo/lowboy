use serde::{Deserialize, Serialize};

#[allow(dead_code)]
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub smtp_relay: String,
    pub smtp_username: String,
    pub smtp_password: String,
}
