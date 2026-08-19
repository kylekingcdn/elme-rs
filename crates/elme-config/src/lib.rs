#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true")]

use config::{Config, Environment};
use dotenvy::dotenv;
use serde::Deserialize;
use std::fmt::Debug;
use thiserror::Error;

const CONFIG_ENV_DELIM: &str = "__";

pub trait ConfigureApp
where
    Self: Debug,
    Self: for<'a> Deserialize<'a>
{
    const ENV_PREFIX: &str;

    /// Loads app config
    ///
    /// Reads from env vars + .env files in run directory
    ///
    /// Uses the prefix configured by
    ///
    /// # Errors
    ///
    /// Returns a [`LoadConfigError`], which provides variants for possible
    /// encountered error types
    fn load() -> Result<Self, LoadConfigError> {
        // load from .env file
        dotenv().ok();

        // init config settings
        let settings = Config::builder()
            .add_source(
                Environment::with_prefix(Self::ENV_PREFIX)
                    .separator(CONFIG_ENV_DELIM)
                    .try_parsing(true),
            )
            .build()?;

        // load config
        let config = settings.try_deserialize()?;

        Ok(config)
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum LoadConfigError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

}
