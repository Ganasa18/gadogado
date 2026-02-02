use crate::domain::error::{AppError, Result};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const TRAINING_SCHEMA_V1: &str = include_str!("../../../../resources/training/schema.sql");

#[derive(Clone)]
pub struct TrainingDb {
    pool: SqlitePool,
}

impl TrainingDb {
    pub async fn connect(db_path: &Path) -> Result<Self> {
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

        // Apply migrations to ensure schema is up to date
        apply_training_migrations(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

async fn apply_training_migrations(pool: &SqlitePool) -> Result<()> {
    // Current schema version. We bump this when we add repair logic.
    const CURRENT_SCHEMA_VERSION: i64 = 4;

    // Check current schema version
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to read training DB user_version: {e}"))
        })?;

    // First, check if corrections table exists at all
    let corrections_exists = check_table_exists(pool, "corrections").await;

    if version < 1 || !corrections_exists {
        // Apply full schema (either fresh DB or missing tables)
        if corrections_exists {
            // Table exists but version is 0 - likely corrupted state, rebuild
            tracing::warn!(
                "Training DB in inconsistent state (version=0 but tables exist), rebuilding..."
            );
            drop_all_tables(pool).await?;
        }
        apply_full_schema(pool).await?;
        let pragma = format!("PRAGMA user_version = {}", CURRENT_SCHEMA_VERSION);
        sqlx::query(&pragma)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to set training DB user_version: {e}"))
            })?;
        return Ok(());
    }

    // Always validate the corrections table (we've seen DBs where user_version is bumped
    // but schema is missing correction_id).
    let mut schema_valid = verify_corrections_schema(pool).await;
    if !schema_valid {
        tracing::warn!(
            "Training DB schema drift detected (missing correction_id); attempting repair..."
        );

        if try_repair_corrections_schema(pool).await? {
            schema_valid = verify_corrections_schema(pool).await;
        }

        if !schema_valid {
            tracing::warn!("Training database schema is corrupted, rebuilding...");
            drop_all_tables(pool).await?;
            apply_full_schema(pool).await?;
        }
    }

    // Bump schema version (even if we didn't change the SQL schema, this encodes "repair logic applied").
    if version < CURRENT_SCHEMA_VERSION {
        let pragma = format!("PRAGMA user_version = {}", CURRENT_SCHEMA_VERSION);
        sqlx::query(&pragma)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to set training DB user_version: {e}"))
            })?;
    }

    Ok(())
}

/// Check if a table exists in the database
async fn check_table_exists(pool: &SqlitePool, table_name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
    )
    .bind(table_name)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
        > 0
}

/// Verify that the corrections table has the expected schema.
///
/// Use PRAGMA table_info instead of sqlite_master.sql so we can detect
/// schema drift even when the CREATE TABLE SQL is missing/rewritten.
async fn verify_corrections_schema(pool: &SqlitePool) -> bool {
    use sqlx::Row;

    let rows = match sqlx::query("PRAGMA table_info(corrections)")
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };

    rows.into_iter().any(|row| {
        let name: String = row.try_get("name").unwrap_or_default();
        name.eq_ignore_ascii_case("correction_id")
    })
}

async fn try_repair_corrections_schema(pool: &SqlitePool) -> Result<bool> {
    use sqlx::Row;

    let rows = match sqlx::query("PRAGMA table_info(corrections)")
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to get corrections table info: {}", e);
            return Ok(false);
        }
    };

    if rows.is_empty() {
        tracing::warn!("corrections table has no columns or doesn't exist");
        return Ok(false);
    }

    let mut has_correction_id = false;
    let mut has_id = false;
    let mut column_names = Vec::new();

    for row in &rows {
        let name: String = row.try_get("name").unwrap_or_default();
        column_names.push(name.clone());
        if name.eq_ignore_ascii_case("correction_id") {
            has_correction_id = true;
        }
        if name.eq_ignore_ascii_case("id") {
            has_id = true;
        }
    }

    tracing::info!(
        "corrections table columns: {:?}, has_correction_id={}, has_id={}",
        column_names,
        has_correction_id,
        has_id
    );

    if has_correction_id {
        // Already has correction_id, no repair needed
        return Ok(false);
    }

    tracing::info!("Attempting to add correction_id column to corrections table");

    // Try best-effort online repair for legacy DBs that used `id` instead of `correction_id`.
    // SQLite can't easily add a PRIMARY KEY after-the-fact, but having the column present
    // unblocks reads and writes.
    match sqlx::query("ALTER TABLE corrections ADD COLUMN correction_id TEXT")
        .execute(pool)
        .await
    {
        Ok(_) => tracing::info!("Successfully added correction_id column"),
        Err(e) => {
            tracing::error!("Failed to add correction_id column: {}", e);
            return Err(AppError::DatabaseError(format!(
                "Failed to repair corrections schema: {e}"
            )));
        }
    }

    if has_id {
        tracing::info!("Backfilling correction_id from id column");
        sqlx::query(
            "UPDATE corrections SET correction_id = id WHERE correction_id IS NULL OR correction_id = ''",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to backfill correction_id from id: {e}"))
        })?;
    }

    // Fill any remaining NULL/empty IDs deterministically.
    tracing::info!("Backfilling any remaining NULL correction_ids");
    sqlx::query(
        "UPDATE corrections SET correction_id = printf('legacy-%s', rowid) WHERE correction_id IS NULL OR correction_id = ''",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        AppError::DatabaseError(format!("Failed to backfill missing correction_id values: {e}"))
    })?;

    // Add a unique index so the app can treat this like a stable identifier.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_corrections_correction_id ON corrections(correction_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to add correction_id index: {e}")))?;

    tracing::info!("corrections schema repair completed successfully");
    Ok(true)
}

/// Drop all tables in the training database
async fn drop_all_tables(pool: &SqlitePool) -> Result<()> {
    let tables = [
        "run_soft_labels",
        "dataset_item_soft_labels",
        "correction_soft_labels",
        "soft_labels",
        "run_artifacts",
        "evaluation_metrics",
        "training_logs",
        "model_actives",
        "model_versions",
        "run_datasets",
        "run_corrections",
        "training_runs",
        "correction_tags",
        "tags",
        "corrections",
        "dataset_items",
        "datasets",
        "models",
    ];

    for table in tables {
        let drop_sql = format!("DROP TABLE IF EXISTS {}", table);
        sqlx::query(&drop_sql)
            .execute(pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to drop table {}: {e}", table)))?;
    }

    Ok(())
}

/// Apply the full V1 schema to the database
async fn apply_full_schema(pool: &SqlitePool) -> Result<()> {
    for statement in TRAINING_SCHEMA_V1.split(';') {
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

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("Training DB path is not valid UTF-8".to_string())
    })?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}
