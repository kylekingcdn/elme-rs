use sqlx::{
    FromRow,
    postgres::{PgPool, PgRow},
};
use std::clone::Clone;
use std::fmt::Debug;
use std::future::Future;

#[cfg(feature = "op-traits")]
pub mod ops;

pub trait Repo: Debug + Clone {
    const TABLE_NAME: &'static str;

    type Row: Send + Unpin + for<'r> FromRow<'r, PgRow>;
    type InsertRow: Send + Unpin + for<'r> FromRow<'r, PgRow>;

    fn new(db: PgPool) -> Self;
    fn db(&self) -> &PgPool;

    #[must_use]
    fn build_bind_index_params(start: usize, count: usize) -> String {
        (start..start + count)
            .map(|i| format!("${i}"))
            .collect::<Vec<String>>()
            .join(", ")
    }

    #[must_use]
    fn build_batch_insert_bind_params(start: usize, fields_per_row: usize, row_count: usize) -> String {
        let mut bind_rows = Vec::new();
        for r in start..row_count {
            let bind_params = Self::build_bind_index_params(r * fields_per_row + 1, fields_per_row);
            bind_rows.push(format!("({bind_params})"));
        }
        bind_rows.join(", ")
    }

    fn count_all(&self) -> impl Future<Output = Result<i64, sqlx::Error>> + Send {
        let db = self.db();
        let query_str = format!(r#"
            SELECT
                COUNT(*)
            FROM
                "public"."{0}"
        "#, Self::TABLE_NAME);

        async move {
            let count = sqlx::query_scalar(&query_str)
                .fetch_one(db)
                .await?;

            Ok(count)
        }
    }

    fn get_all(&self) -> impl Future<Output = Result<Vec<Self::Row>, sqlx::Error>> + Send {
        let db = self.db();
        let query_str = format!(r#"
            SELECT
                *
            FROM
                "public"."{0}"
        "#, Self::TABLE_NAME);

        async move {
            let rows = sqlx::query_as(&query_str)
                .fetch_all(db)
                .await?;

            Ok(rows)
        }
    }
}
