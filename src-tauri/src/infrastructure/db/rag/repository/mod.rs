use crate::domain::error::{AppError, Result};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

mod chunks;
mod collections;
mod context_settings;
mod db_connector;
mod documents;
mod entities;
mod excel;
mod quality;
mod structured_rows;

pub use chunks::{ChunkWithMetadata, ChunkWithMetadataScore};
pub use structured_rows::StructuredRowWithDoc;

pub struct RagRepository {
    pool: SqlitePool,
}

impl RagRepository {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let db_url = db_path_to_url(db_path)?;
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse RAG DB URL: {}", e)))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to connect RAG DB: {}", e)))?;

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool for direct queries
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("RAG database path is not valid UTF-8".to_string())
    })?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}
