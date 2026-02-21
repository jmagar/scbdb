use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfilerError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("database error: {0}")]
    Db(#[from] scbdb_db::DbError),

    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}
