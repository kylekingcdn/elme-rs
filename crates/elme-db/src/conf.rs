use secrecy::SecretString;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct ManagedDbConf {
    /// The database connection URL
    pub url: SecretString, // mandatory

    #[serde(flatten)]
    pub pool: PoolConf,

    /// Toggles automatic database migrations during app startup
    #[serde(default = "ManagedDbConf::migrate_default")]
    pub migrate: bool,
}
impl ManagedDbConf {
    #[must_use]
    pub fn migrate_default() -> bool {
        false
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnmanagedDbConf {
    /// The database connection URL
    pub url: SecretString, // mandatory

    #[serde(flatten)]
    pub pool: PoolConf,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
pub struct PoolConf {
    pub min_connections: Option<u32>,
    pub max_connections: Option<u32>,

    pub acquire_slow_threshold: Option<u64>,
    pub acquire_timeout: Option<u64>,
    pub max_lifetime: Option<u64>,
    pub idle_timeout: Option<u64>,
}
impl From<PoolConf> for PgPoolOptions {
    fn from(conf: PoolConf) -> Self {
        let mut opts = Self::new();

        if let Some(val) = conf.min_connections {
            opts = opts.min_connections(val);
        }
        if let Some(val) = conf.max_connections {
            opts = opts.max_connections(val);
        }
        if let Some(val) = conf.acquire_slow_threshold {
            opts = opts.acquire_slow_threshold(Duration::from_secs(val));
        }
        if let Some(val) = conf.acquire_timeout {
            opts = opts.acquire_timeout(Duration::from_secs(val));
        }
        if let Some(val) = conf.max_lifetime {
            opts = opts.max_lifetime(Duration::from_secs(val));
        }
        if let Some(val) = conf.idle_timeout {
            opts = opts.idle_timeout(Duration::from_secs(val));
        }
        opts
    }
}
