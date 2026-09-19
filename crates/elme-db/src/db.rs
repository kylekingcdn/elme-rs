use crate::{
    conf::PgPoolConfig,
    repo::Repo,
};
use sqlx::{
    migrate::{MigrateError, Migrator},
    postgres::PgPool,
};
use std::fmt::Debug;
use std::marker::PhantomData;

// !- Management scope

/// Marker trait for managed/unmanaged states
pub trait ManagementScope: Debug + Copy + Clone {}

// ! Unmanaged scope

#[derive(Debug, Copy, Clone)]
pub struct Unmanaged;

impl ManagementScope for Unmanaged {}

// ! Managed scope

#[derive(Debug, Copy, Clone)]
pub struct Managed;

impl ManagementScope for Managed {}

// !- Db struct

#[derive(Debug, Clone)]
pub struct Db<T: ManagementScope> {
    pool: PgPool,
    _scope: PhantomData<T>,
}

impl<T: ManagementScope> Db<T> {
    #[must_use]
    fn new_shared(conf: PgPoolConfig) -> Self {
        let url = conf.url_exposed().expect("Database URL is set");
        let pool_opts = conf.into_pool_options();
        let pool = pool_opts
            .connect_lazy(&url)
            .expect("Database URL is valid");

        Self {
            pool,
            _scope: PhantomData
        }
    }

    /// Returns the underlying [`PgPool`]
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ! TODO: add repo generic param for mutability
    /// Returns the type-inferred repository
    #[must_use]
    pub fn repo<R: Repo>(&self) -> R {
        R::new(self.pool.clone())
    }
}

// ! Unmanaged Db

pub type UnmanagedDb = Db<Unmanaged>;

impl UnmanagedDb {
    /// Constructs a new [`UnmanagedDb`]
    ///
    /// # Panics
    ///
    /// Panics if the configuration's database url is invalid or missing
    #[must_use]
    pub fn new(conf: PgPoolConfig) -> Self {
        Self::new_shared(conf)
    }
}

// ! Managed Db

pub type ManagedDb = Db<Managed>;

impl ManagedDb {
    /// Constructs a new [`ManagedDb`]
    ///
    /// # Panics
    ///
    /// Panics if the configuration's database url is invalid or missing
    #[must_use]
    pub fn new(conf: PgPoolConfig) -> Self {
        Self::new_shared(conf)
    }

    /// Builds a new [`ManagedDb`], executing migrations if the config value for `migrate` is true
    ///
    /// ---
    ///
    /// The `migrator` param can be resolved by calling the following from the
    /// integrating bin crate:
    ///
    /// ```rust,ignore
    /// sqlx::migrate!() // path defaults to ./migrations
    /// ```
    ///
    /// or with a custom path:
    ///
    /// ```rust,ignore
    /// sqlx::migrate!("./db/migrations")
    /// ```
    ///
    /// # Errors
    ///
    /// Passes through any encountered [`MigrateError`]'s
    pub async fn new_migrated(
        conf: PgPoolConfig,
        migrator: Migrator,
    ) -> Result<Self, MigrateError> {
        let migrate = conf.migrate();
        let db = Self::new(conf);

        if migrate {
            db.migrate(migrator).await?;
        } else {
            tracing::info!("Skipping migrations");
        }

        Ok(db)
    }

    /// Executes migrations
    ///
    /// ---
    ///
    /// The `migrator` param can be resolved by calling the following from the
    /// integrating bin crate:
    ///
    /// ```rust,ignore
    /// sqlx::migrate!() // path defaults to ./migrations
    /// ```
    ///
    /// or with a custom path:
    ///
    /// ```rust,ignore
    /// sqlx::migrate!("./db/migrations")
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
        migrator.run(self.pool()).await?;

        Ok(())
    }
}
