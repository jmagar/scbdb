use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brand {
    pub id: Uuid,
    pub name: String,
    pub relationship: String,
    pub tier: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid relationship: {0}")]
    InvalidRelationship(String),
}
