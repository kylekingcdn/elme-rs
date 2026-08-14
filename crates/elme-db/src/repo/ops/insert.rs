use crate::repo::Repo;

use sqlx::{
    postgres::{PgArguments, Postgres},
    query::Query,
};
use tracing::instrument;

// !- Insert one

pub trait PrepInsertOne: Repo {
    fn insert_one_statement() -> String;
    fn prepare_insert_one(statement: &str, row: <Self as Repo>::InsertRow) -> Query<'_, Postgres, PgArguments>;
}
pub trait InsertOne: Repo + PrepInsertOne {
    #[instrument(level="debug", skip_all, err, fields(table=Self::TABLE_NAME))]
    fn insert_one(&self, row: <Self as Repo>::InsertRow) -> impl Future<Output = Result<(), sqlx::Error>> + Send {
        let db = self.db();
        async move {
            tracing::debug!(table=Self::TABLE_NAME, "Inserting row");

            let statement = Self::insert_one_statement();
            let query = Self::prepare_insert_one(&statement, row);

            query.execute(db).await?;

            Ok(())
        }
    }
}
impl<R> InsertOne for R
where
    R: Repo + PrepInsertOne
{}

// !- Insert many

pub trait PrepInsertMany: Repo {
    fn insert_many_statement(row_count: usize) -> String;
    fn prepare_insert_many(statement: &str, rows: Vec<<Self as Repo>::InsertRow>) -> Query<'_, Postgres, PgArguments>;
}
pub trait InsertMany: Repo + PrepInsertMany {
    #[instrument(level="debug", skip_all, err, fields(table=Self::TABLE_NAME, row_count=rows.len()))]
    fn insert_many(&self, rows: Vec<<Self as Repo>::InsertRow>) -> impl Future<Output = Result<(), sqlx::Error>> + Send {
        let db = self.db();
        async move {
            if rows.is_empty() {
                tracing::debug!(table=Self::TABLE_NAME, "Skipping empty insert");
                return Ok(());
            }
            tracing::debug!(row_count=?rows.len(), table=Self::TABLE_NAME, "Inserting rows");

            let statement = Self::insert_many_statement(rows.len());
            let query = Self::prepare_insert_many(&statement, rows);

            query.execute(db).await?;

            Ok(())
        }
    }
}
impl<R> InsertMany for R
where
    R: Repo + PrepInsertMany
{}
