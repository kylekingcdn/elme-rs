use sqlx::{
    postgres::{PgArguments, PgPool, Postgres},
    query::Query
};
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct PartialVec<T> {
    pub inner: Vec<T>,
    pub total_items: usize,
}
impl<T> PartialVec<T> {
    #[must_use]
    pub fn new(items: Vec<T>, total_items: usize) -> Self {
        Self {
            inner: items,
            total_items,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}
impl SortOrder {
    #[must_use]
    pub fn sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
    #[must_use]
    pub fn invert(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

/// Executes each query sequentially, aborting on the first error
///
/// Primary use-case is providing the logic for fail-fast tx rollback
///
/// # Errors
///
/// Passes through the first encountered [`sqlx::Error`], if any
#[instrument(skip_all, err, fields(query_count=queries.len()))]
pub async fn execute_queries(
    db: &PgPool,
    queries: Vec<Query<'_, Postgres, PgArguments>>,
) -> Result<(), sqlx::Error> {
    for query in queries {
        query.execute(db).await?;
    }
    Ok(())
}

/// Executes each query sequentially, aborting on the first errors.
///
/// Wraps the execution chain in a tx. All queries in the chain are rolled back upon failure.
///
/// # Errors
///
/// Passes through the first encountered [`sqlx::Error`], if any
#[instrument(skip_all, err, fields(query_count=queries.len()))]
pub async fn execute_queries_with_tx(
    db: &PgPool,
    queries: Vec<Query<'_, Postgres, PgArguments>>,
) -> Result<(), sqlx::Error> {
    // ! TODO: add debug chain queries feature flag for opt-in logging of specific failure sql
    // let mut queries_sql = Vec::new();
    // for query in &queries {
    //     queries_sql.push(query.sql());
    // }
    let tx = db.begin().await?;
    match execute_queries(db, queries).await {
        Ok(()) => {
            tx.commit().await?;
            Ok(())
        },
        Err(error) => {
            tx.rollback().await?;
            tracing::error!(?error, "Failed to execute query chain, queries were rolled back");
            // tracing::error!("Queries: {queries_sql:#?}");
            Err(error)?
        }
    }
}
