use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("{name}{id_text} not found")]
    EntityNotFound {
        name: String,
        id: Option<String>,
        id_text: String,
    },

    #[error("Unhandled SQLx error occurred: {0:?}")]
    SqlxError(#[from] sqlx::Error),
}
impl DbError {
    pub fn map_not_found(error: sqlx::Error, entity_name: &str, entity_id: Option<String>) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::EntityNotFound {
                id_text: entity_id.as_ref().map_or_else(String::new, |id| format!(" ({id})")),
                name: entity_name.to_string(),
                id: entity_id,
            },
            _ => Self::SqlxError(error),
        }
    }
}
