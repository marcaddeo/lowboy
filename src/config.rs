#![allow(dead_code)]
use anyhow::{Context as _, Result};
use confique::{yaml::FormatOptions, Config as _};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::auth::IdentityProviderConfig;

#[derive(Clone, Debug, Serialize, Deserialize, confique::Config)]
pub struct Config {
    /// Database url
    pub database_url: String,

    /// Database connection pool size
    #[config(default = 16)]
    pub database_pool_size: usize,

    /// Base64 encoded session key
    #[config(env = "LOWBOY_SESSION_KEY")]
    pub session_key: String,

    // OAuth Provider Configuration
    pub oauth_providers: Vec<IdentityProviderConfig>,
}

impl Config {
    pub fn load(config_path: Option<PathBuf>) -> Result<Config> {
        let config_path = get_config_path(config_path)?;
        let config = Config::builder().env().file(config_path).load()?;

        Ok(config)
    }
}

pub fn get_config_template() -> String {
    confique::yaml::template::<Config>(FormatOptions::default())
}

pub fn print_config_template() {
    println!("{}", get_config_template());
}

pub fn get_config_path(config_path: Option<PathBuf>) -> Result<PathBuf> {
    match config_path {
        Some(path) => Ok(path),
        None => {
            let xdg_dirs = xdg::BaseDirectories::with_prefix("lowboy")?;
            Ok(xdg_dirs.get_config_file("config.yml"))
        }
    }
}

pub fn write_config_template(config_path: Option<PathBuf>) -> Result<PathBuf> {
    let config_path = get_config_path(config_path)?;
    let config_template = get_config_template();

    let config_path_dir = config_path
        .parent()
        .context("Could not determine config file parent dir")?;

    std::fs::create_dir_all(config_path_dir)?;
    std::fs::write(config_path.clone(), config_template)?;

    Ok(config_path)
}
