use crate::{
    conf::{ManagedDbConf, UnmanagedDbConf},
    repo::Repo,
};
use secrecy::ExposeSecret;
use sqlx::{
    migrate::{MigrateError, Migrator},
    postgres::{PgPool, PgPoolOptions},
};

#[derive(Debug, Clone)]
pub struct UnmanagedDb {
    pool: PgPool,
}
impl UnmanagedDb {
    /// Constructs a new [`UnmanagedDb`]
    ///
    /// # Panics
    ///
    /// Panics if the configuration contains an invalid database url
    #[must_use]
    pub fn new(conf: &UnmanagedDbConf) -> Self {
        let pool_opts = PgPoolOptions::from(conf.pool);
        let pool = pool_opts
            .connect_lazy(conf.url.expose_secret())
            .expect("Valid database URL");

        Self {
            pool,
        }
    }
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    #[must_use]
    pub fn repo<R: Repo>(&self) -> R {
        R::new(self.pool.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ManagedDb {
    pool: PgPool,
}
impl ManagedDb {
    /// Constructs a new [`ManagedDb`]
    ///
    /// # Panics
    ///
    /// Panics if the configuration contains an invalid database url
    #[must_use]
    pub fn new(conf: &ManagedDbConf) -> Self {
        let pool_opts = PgPoolOptions::from(conf.pool);
        let pool = pool_opts
            .connect_lazy(conf.url.expose_secret())
            .expect("Valid database URL");

        Self {
            pool,
        }
    }
    /// Builds a new [`ManagedDb`], executing migrations if the config value for `migrate` is true
    ///
    /// ---
    /// The `migrator` param can be resolved by calling the following from the calling bin crate
    /// ```rust,ignore
    /// sqlx::migrate!("./migration")
    /// ```
    ///
    /// # Errors
    ///
    /// Passes through any encountered [`MigrateError`]'s
    pub async fn new_migrated(conf: &ManagedDbConf, migrator: Migrator) -> Result<Self, MigrateError> {
        let db = Self::new(conf);
        if conf.migrate {
            db.migrate(migrator).await?;
        } else {
            tracing::info!("Skipping migrations");
        }

        Ok(db)
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    #[must_use]
    pub fn repo<R: Repo>(&self) -> R {
        R::new(self.pool.clone())
    }

    /// Executes migrations
    ///
    /// ---
    /// The `migrator` param can be resolved by calling the following from the calling bin crate
    /// ```rust,ignore
    /// sqlx::migrate!("./migration")
    /// ```
    ///
    /// # Errors
    ///
    /// Passes through any encountered [`MigrateError`]'s
    pub async fn migrate(&self, migrator: Migrator) -> Result<(), MigrateError> {
        let connect_opts = self.pool.connect_options();
        let db_host = connect_opts.get_host();
        let db_database = connect_opts.get_database();

        tracing::info!(?db_host, ?db_database, "Running migrations");
        migrator.run(&self.pool).await?;

        Ok(())
    }
}
