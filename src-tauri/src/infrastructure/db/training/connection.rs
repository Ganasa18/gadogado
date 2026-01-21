use crate::domain::error::{AppError, Result};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const TRAINING_SCHEMA_V1: &str = include_str!("../../../resources/training/schema.sql");

pub async fn init_training_db(db_path: &Path) -> Result<()> {
    let db_url = db_path_to_url(db_path)?;
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| AppError::DatabaseError(format!("Failed to parse training DB URL: {e}")))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to connect training DB: {e}")))?;

    apply_migrations(&pool).await?;

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Training DB health check failed: {e}")))?;

    Ok(())
}

pub async fn connect_training_pool(db_path: &Path) -> Result<SqlitePool> {
    let db_url = db_path_to_url(db_path)?;
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| AppError::DatabaseError(format!("Failed to parse training DB URL: {e}")))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to connect training DB: {e}")))?;

    Ok(pool)
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("Training DB path is not valid UTF-8".to_string())
    })?;

    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

async fn apply_migrations(pool: &SqlitePool) -> Result<()> {
    // Minimal migration mechanism:
    // - Use PRAGMA user_version for schema version.
    // - v1 == schema.sql (current full schema).
    // - Future migrations should increment user_version and apply incremental statements.

    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to read training DB user_version: {e}"))
        })?;

    if version < 1 {
        apply_schema(pool, TRAINING_SCHEMA_V1).await?;
        sqlx::query("PRAGMA user_version = 1")
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to set training DB user_version: {e}"))
            })?;
    }

    Ok(())
}

async fn apply_schema(pool: &SqlitePool, schema: &str) -> Result<()> {
    for statement in schema.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt).execute(pool).await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to apply training schema: {e}"))
        })?;
    }
    Ok(())
}
