use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub enum AppError {
    Internal(String),
    NotFound(String),
    ValidationError(String),
    LLMError(String),
    SecurityError(String),
    DatabaseError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::LLMError(msg) => write!(f, "LLM error: {}", msg),
            AppError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
