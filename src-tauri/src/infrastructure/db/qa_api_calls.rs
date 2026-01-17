use crate::domain::error::{AppError, Result};
use crate::domain::qa_api_call::QaApiCall;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub struct QaApiCallRepository {
    pool: SqlitePool,
}

impl QaApiCallRepository {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let db_url = db_path_to_url(db_path)?;
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse QA DB URL: {e}")))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to connect QA DB: {e}")))?;

        Ok(Self { pool })
    }

    pub async fn insert_call(&self, call: &QaApiCall) -> Result<()> {
        sqlx::query(
            "INSERT INTO api_calls (id, session_id, run_id, event_request_id, event_response_id, method, url, request_headers_json, request_body_json, request_body_hash, response_status, response_headers_json, response_body_hash, timing_ms, created_at)
             VALUES (?, ?, ?, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&call.id)
        .bind(&call.session_id)
        .bind(&call.run_id)
        .bind(&call.method)
        .bind(&call.url)
        .bind(&call.request_headers_json)
        .bind(&call.request_body_json)
        .bind(&call.request_body_hash)
        .bind(call.response_status)
        .bind(&call.response_headers_json)
        .bind(&call.response_body_hash)
        .bind(call.timing_ms)
        .bind(call.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert API call: {e}")))?;

        Ok(())
    }
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("QA database path is not valid UTF-8".to_string())
    })?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}
