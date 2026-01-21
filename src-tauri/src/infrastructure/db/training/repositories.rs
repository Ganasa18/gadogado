use crate::domain::error::{AppError, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingMethod {
    FineTune,
    KnowledgeDistillation,
    Hybrid,
}

impl TrainingMethod {
    fn as_db(&self) -> &'static str {
        match self {
            TrainingMethod::FineTune => "fine_tune",
            TrainingMethod::KnowledgeDistillation => "knowledge_distillation",
            TrainingMethod::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    RolledBack,
}

impl TrainingStatus {
    fn as_db(&self) -> &'static str {
        match self {
            TrainingStatus::Queued => "queued",
            TrainingStatus::Running => "running",
            TrainingStatus::Completed => "completed",
            TrainingStatus::Failed => "failed",
            TrainingStatus::Cancelled => "cancelled",
            TrainingStatus::RolledBack => "rolled_back",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    pub correction_id: String,
    pub prompt: String,
    pub student_output: String,
    pub corrected_output: String,
    pub accuracy_rating: i64,
    pub relevance_rating: Option<i64>,
    pub safety_rating: Option<i64>,
    pub domain_notes: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionInput {
    pub correction_id: String,
    pub prompt: String,
    pub student_output: String,
    pub corrected_output: String,
    pub accuracy_rating: i64,
    pub relevance_rating: Option<i64>,
    pub safety_rating: Option<i64>,
    pub domain_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingRun {
    pub run_id: String,
    pub student_model_id: String,
    pub base_version_id: Option<String>,
    pub teacher_model_id: Option<String>,
    pub method: String,
    pub status: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub hyperparams_json: String,
    pub seed: Option<i64>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingRunInput {
    pub run_id: String,
    pub student_model_id: String,
    pub base_version_id: Option<String>,
    pub teacher_model_id: Option<String>,
    pub method: TrainingMethod,
    pub hyperparams_json: String,
    pub seed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersion {
    pub version_id: String,
    pub model_id: String,
    pub run_id: Option<String>,
    pub parent_version_id: Option<String>,
    pub created_at: Option<String>,
    pub is_promoted: bool,
    pub promoted_at: Option<String>,
    pub artifact_path: String,
    pub artifact_hash: Option<String>,
    pub artifact_size_bytes: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionInput {
    pub version_id: String,
    pub model_id: String,
    pub run_id: Option<String>,
    pub parent_version_id: Option<String>,
    pub artifact_path: String,
    pub artifact_hash: Option<String>,
    pub artifact_size_bytes: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingLogInput {
    pub run_id: String,
    pub epoch: i64,
    pub step: i64,
    pub loss: Option<f64>,
    pub lr: Option<f64>,
    pub temperature: Option<f64>,
    pub cpu_util: Option<f64>,
    pub ram_usage_mb: Option<i64>,
    pub gpu_util: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationMetricInput {
    pub version_id: String,
    pub dataset_id: String,
    pub metric_name: String,
    pub metric_value: f64,
}

const TRAINING_SCHEMA_V1: &str = include_str!("../../../resources/training/schema.sql");

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
            tracing::warn!("Training DB in inconsistent state (version=0 but tables exist), rebuilding...");
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
        tracing::warn!("Training DB schema drift detected (missing correction_id); attempting repair...");

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
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_corrections_correction_id ON corrections(correction_id)")
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to add correction_id index: {e}"))
        })?;

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

pub struct CorrectionRepository {
    pool: SqlitePool,
}

impl CorrectionRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, correction: &CorrectionInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO corrections (correction_id, prompt, student_output, corrected_output, accuracy_rating, relevance_rating, safety_rating, domain_notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&correction.correction_id)
        .bind(&correction.prompt)
        .bind(&correction.student_output)
        .bind(&correction.corrected_output)
        .bind(correction.accuracy_rating)
        .bind(correction.relevance_rating)
        .bind(correction.safety_rating)
        .bind(&correction.domain_notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert correction: {e}")))?;
        Ok(())
    }

    pub async fn get(&self, correction_id: &str) -> Result<Correction> {
        let row = sqlx::query_as::<_, CorrectionEntity>(
            "SELECT correction_id, prompt, student_output, corrected_output, accuracy_rating, relevance_rating, safety_rating, domain_notes, created_at \
             FROM corrections WHERE correction_id = ?",
        )
        .bind(correction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch correction: {e}")))?;

        match row {
            Some(entity) => Ok(entity.into()),
            None => Err(AppError::NotFound(format!(
                "Correction not found: {}",
                correction_id
            ))),
        }
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<Correction>> {
        let rows = sqlx::query_as::<_, CorrectionEntity>(
            "SELECT correction_id, prompt, student_output, corrected_output, accuracy_rating, relevance_rating, safety_rating, domain_notes, created_at \
             FROM corrections ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list corrections: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn delete(&self, correction_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM corrections WHERE correction_id = ?")
            .bind(correction_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete correction: {e}")))?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct CorrectionEntity {
    correction_id: String,
    prompt: String,
    student_output: String,
    corrected_output: String,
    accuracy_rating: i64,
    relevance_rating: Option<i64>,
    safety_rating: Option<i64>,
    domain_notes: Option<String>,
    created_at: String,
}

impl From<CorrectionEntity> for Correction {
    fn from(entity: CorrectionEntity) -> Self {
        Self {
            correction_id: entity.correction_id,
            prompt: entity.prompt,
            student_output: entity.student_output,
            corrected_output: entity.corrected_output,
            accuracy_rating: entity.accuracy_rating,
            relevance_rating: entity.relevance_rating,
            safety_rating: entity.safety_rating,
            domain_notes: entity.domain_notes,
            created_at: Some(entity.created_at),
        }
    }
}

pub struct TrainingRunRepository {
    pool: SqlitePool,
}

impl TrainingRunRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, run: &TrainingRunInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO training_runs (run_id, student_model_id, base_version_id, teacher_model_id, method, status, hyperparams_json, seed) \
             VALUES (?, ?, ?, ?, ?, 'queued', ?, ?)",
        )
        .bind(&run.run_id)
        .bind(&run.student_model_id)
        .bind(&run.base_version_id)
        .bind(&run.teacher_model_id)
        .bind(run.method.as_db())
        .bind(&run.hyperparams_json)
        .bind(run.seed)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert training run: {e}")))?;
        Ok(())
    }

    pub async fn set_status(
        &self,
        run_id: &str,
        status: TrainingStatus,
        end_time: Option<String>,
        failure_reason: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE training_runs SET status = ?, end_time = COALESCE(?, end_time), failure_reason = ? WHERE run_id = ?",
        )
        .bind(status.as_db())
        .bind(end_time)
        .bind(failure_reason)
        .bind(run_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update training run: {e}")))?;

        Ok(())
    }

    pub async fn get(&self, run_id: &str) -> Result<TrainingRun> {
        let run = sqlx::query_as::<_, TrainingRunEntity>(
            "SELECT run_id, student_model_id, base_version_id, teacher_model_id, method, status, start_time, end_time, hyperparams_json, seed, failure_reason \
             FROM training_runs WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch training run: {e}")))?;

        match run {
            Some(entity) => Ok(entity.into()),
            None => Err(AppError::NotFound(format!(
                "Training run not found: {}",
                run_id
            ))),
        }
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<TrainingRun>> {
        let rows = sqlx::query_as::<_, TrainingRunEntity>(
            "SELECT run_id, student_model_id, base_version_id, teacher_model_id, method, status, start_time, end_time, hyperparams_json, seed, failure_reason \
             FROM training_runs ORDER BY start_time DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list training runs: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct TrainingRunEntity {
    run_id: String,
    student_model_id: String,
    base_version_id: Option<String>,
    teacher_model_id: Option<String>,
    method: String,
    status: String,
    start_time: String,
    end_time: Option<String>,
    hyperparams_json: String,
    seed: Option<i64>,
    failure_reason: Option<String>,
}

impl From<TrainingRunEntity> for TrainingRun {
    fn from(entity: TrainingRunEntity) -> Self {
        Self {
            run_id: entity.run_id,
            student_model_id: entity.student_model_id,
            base_version_id: entity.base_version_id,
            teacher_model_id: entity.teacher_model_id,
            method: entity.method,
            status: entity.status,
            start_time: Some(entity.start_time),
            end_time: entity.end_time,
            hyperparams_json: entity.hyperparams_json,
            seed: entity.seed,
            failure_reason: entity.failure_reason,
        }
    }
}

pub struct RunCorrectionsRepository {
    pool: SqlitePool,
}

impl RunCorrectionsRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn add(
        &self,
        run_id: &str,
        correction_id: &str,
        split: &str,
        weight: f64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_corrections (run_id, correction_id, split, weight) VALUES (?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(correction_id)
        .bind(split)
        .bind(weight)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to attach correction to run: {e}"))
        })?;
        Ok(())
    }

    pub async fn list_for_run(&self, run_id: &str) -> Result<Vec<(String, String, f64)>> {
        let rows = sqlx::query_as::<_, RunCorrectionEntity>(
            "SELECT correction_id, split, weight FROM run_corrections WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list run corrections: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|row| (row.correction_id, row.split, row.weight))
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct RunCorrectionEntity {
    correction_id: String,
    split: String,
    weight: f64,
}

pub struct ModelVersionRepository {
    pool: SqlitePool,
}

impl ModelVersionRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, version: &ModelVersionInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO model_versions (version_id, model_id, run_id, parent_version_id, artifact_path, artifact_hash, artifact_size_bytes, notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&version.version_id)
        .bind(&version.model_id)
        .bind(&version.run_id)
        .bind(&version.parent_version_id)
        .bind(&version.artifact_path)
        .bind(&version.artifact_hash)
        .bind(version.artifact_size_bytes)
        .bind(&version.notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert model version: {e}")))?;

        Ok(())
    }

    pub async fn get(&self, version_id: &str) -> Result<ModelVersion> {
        let row = sqlx::query_as::<_, ModelVersionEntity>(
            "SELECT version_id, model_id, run_id, parent_version_id, created_at, is_promoted, promoted_at, artifact_path, artifact_hash, artifact_size_bytes, notes \
             FROM model_versions WHERE version_id = ?",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch model version: {e}")))?;

        match row {
            Some(entity) => Ok(entity.into()),
            None => Err(AppError::NotFound(format!(
                "Model version not found: {}",
                version_id
            ))),
        }
    }

    pub async fn list_by_model(&self, model_id: &str) -> Result<Vec<ModelVersion>> {
        let rows = sqlx::query_as::<_, ModelVersionEntity>(
            "SELECT version_id, model_id, run_id, parent_version_id, created_at, is_promoted, promoted_at, artifact_path, artifact_hash, artifact_size_bytes, notes \
             FROM model_versions WHERE model_id = ? ORDER BY created_at DESC",
        )
        .bind(model_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list model versions: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn list_all(&self) -> Result<Vec<ModelVersion>> {
        let rows = sqlx::query_as::<_, ModelVersionEntity>(
            "SELECT version_id, model_id, run_id, parent_version_id, created_at, is_promoted, promoted_at, artifact_path, artifact_hash, artifact_size_bytes, notes \
             FROM model_versions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list model versions: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn find_by_run_id(&self, run_id: &str) -> Result<Option<ModelVersion>> {
        let row = sqlx::query_as::<_, ModelVersionEntity>(
            "SELECT version_id, model_id, run_id, parent_version_id, created_at, is_promoted, promoted_at, artifact_path, artifact_hash, artifact_size_bytes, notes \
             FROM model_versions WHERE run_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch model version by run: {e}")))?;

        Ok(row.map(|e| e.into()))
    }

    pub async fn verify_artifact_exists(&self, version_id: &str) -> Result<bool> {
        let version = self.get(version_id).await?;
        let path = std::path::Path::new(&version.artifact_path);
        Ok(path.exists())
    }

    pub async fn get_previous_versions(
        &self,
        model_id: &str,
        before_version_id: &str,
        limit: i64,
    ) -> Result<Vec<ModelVersion>> {
        let current = self.get(before_version_id).await?;
        let current_created_at = current.created_at.unwrap_or_default();

        let rows = sqlx::query_as::<_, ModelVersionEntity>(
            "SELECT version_id, model_id, run_id, parent_version_id, created_at, is_promoted, promoted_at, artifact_path, artifact_hash, artifact_size_bytes, notes \
             FROM model_versions WHERE model_id = ? AND created_at < ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(model_id)
        .bind(&current_created_at)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list previous model versions: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ModelVersionEntity {
    version_id: String,
    model_id: String,
    run_id: Option<String>,
    parent_version_id: Option<String>,
    created_at: String,
    is_promoted: i64,
    promoted_at: Option<String>,
    artifact_path: String,
    artifact_hash: Option<String>,
    artifact_size_bytes: Option<i64>,
    notes: Option<String>,
}

impl From<ModelVersionEntity> for ModelVersion {
    fn from(entity: ModelVersionEntity) -> Self {
        Self {
            version_id: entity.version_id,
            model_id: entity.model_id,
            run_id: entity.run_id,
            parent_version_id: entity.parent_version_id,
            created_at: Some(entity.created_at),
            is_promoted: entity.is_promoted != 0,
            promoted_at: entity.promoted_at,
            artifact_path: entity.artifact_path,
            artifact_hash: entity.artifact_hash,
            artifact_size_bytes: entity.artifact_size_bytes,
            notes: entity.notes,
        }
    }
}

pub struct ActiveModelRepository {
    pool: SqlitePool,
}

impl ActiveModelRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn set_active(&self, model_id: &str, version_id: &str) -> Result<()> {
        // SQLite UPSERT.
        sqlx::query(
            "INSERT INTO model_actives (model_id, version_id) VALUES (?, ?) \
             ON CONFLICT(model_id) DO UPDATE SET version_id = excluded.version_id, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(model_id)
        .bind(version_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to set active model: {e}")))?;

        Ok(())
    }

    pub async fn get_active_version_id(&self, model_id: &str) -> Result<Option<String>> {
        let value = sqlx::query_scalar::<_, Option<String>>(
            "SELECT version_id FROM model_actives WHERE model_id = ?",
        )
        .bind(model_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get active model version: {e}")))?;

        Ok(value.flatten())
    }
}

pub struct TrainingLogRepository {
    pool: SqlitePool,
}

impl TrainingLogRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, log: &TrainingLogInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO training_logs (run_id, epoch, step, loss, lr, temperature, cpu_util, ram_usage_mb, gpu_util) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.run_id)
        .bind(log.epoch)
        .bind(log.step)
        .bind(log.loss)
        .bind(log.lr)
        .bind(log.temperature)
        .bind(log.cpu_util)
        .bind(log.ram_usage_mb)
        .bind(log.gpu_util)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert training log: {e}")))?;

        Ok(())
    }

    pub async fn list_for_run(&self, run_id: &str, limit: i64) -> Result<Vec<TrainingLog>> {
        let mut rows = sqlx::query_as::<_, TrainingLogEntity>(
            "SELECT log_id, run_id, epoch, step, loss, lr, temperature, cpu_util, ram_usage_mb, gpu_util, timestamp \
             FROM training_logs WHERE run_id = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list training logs: {e}")))?;

        // Return in chronological order for easier charting.
        rows.reverse();
        Ok(rows.into_iter().map(|e| e.into()).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingLog {
    pub log_id: i64,
    pub run_id: String,
    pub epoch: i64,
    pub step: i64,
    pub loss: Option<f64>,
    pub lr: Option<f64>,
    pub temperature: Option<f64>,
    pub cpu_util: Option<f64>,
    pub ram_usage_mb: Option<i64>,
    pub gpu_util: Option<f64>,
    pub timestamp: String,
}

#[derive(sqlx::FromRow)]
struct TrainingLogEntity {
    log_id: i64,
    run_id: String,
    epoch: i64,
    step: i64,
    loss: Option<f64>,
    lr: Option<f64>,
    temperature: Option<f64>,
    cpu_util: Option<f64>,
    ram_usage_mb: Option<i64>,
    gpu_util: Option<f64>,
    timestamp: String,
}

impl From<TrainingLogEntity> for TrainingLog {
    fn from(entity: TrainingLogEntity) -> Self {
        Self {
            log_id: entity.log_id,
            run_id: entity.run_id,
            epoch: entity.epoch,
            step: entity.step,
            loss: entity.loss,
            lr: entity.lr,
            temperature: entity.temperature,
            cpu_util: entity.cpu_util,
            ram_usage_mb: entity.ram_usage_mb,
            gpu_util: entity.gpu_util,
            timestamp: entity.timestamp,
        }
    }
}

pub struct EvaluationMetricsRepository {
    pool: SqlitePool,
}

impl EvaluationMetricsRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn upsert(&self, metric: &EvaluationMetricInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO evaluation_metrics (version_id, dataset_id, metric_name, metric_value) VALUES (?, ?, ?, ?) \
             ON CONFLICT(version_id, dataset_id, metric_name) DO UPDATE SET metric_value = excluded.metric_value, evaluated_at = CURRENT_TIMESTAMP",
        )
        .bind(&metric.version_id)
        .bind(&metric.dataset_id)
        .bind(&metric.metric_name)
        .bind(metric.metric_value)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to upsert evaluation metric: {e}")))?;

        Ok(())
    }

    pub async fn list_for_version(&self, version_id: &str) -> Result<Vec<EvaluationMetric>> {
        let rows = sqlx::query_as::<_, EvaluationMetricEntity>(
            "SELECT metric_id, version_id, dataset_id, metric_name, metric_value, evaluated_at \
             FROM evaluation_metrics WHERE version_id = ?",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list evaluation metrics: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationMetric {
    pub metric_id: i64,
    pub version_id: String,
    pub dataset_id: String,
    pub metric_name: String,
    pub metric_value: f64,
    pub evaluated_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EvaluationMetricEntity {
    metric_id: i64,
    version_id: String,
    dataset_id: String,
    metric_name: String,
    metric_value: f64,
    evaluated_at: String,
}

impl From<EvaluationMetricEntity> for EvaluationMetric {
    fn from(entity: EvaluationMetricEntity) -> Self {
        Self {
            metric_id: entity.metric_id,
            version_id: entity.version_id,
            dataset_id: entity.dataset_id,
            metric_name: entity.metric_name,
            metric_value: entity.metric_value,
            evaluated_at: Some(entity.evaluated_at),
        }
    }
}

// ============================================================================
// Tag Repository
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub tag_id: i64,
    pub name: String,
}

pub struct TagRepository {
    pool: SqlitePool,
}

impl TagRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn get_or_create(&self, name: &str) -> Result<i64> {
        // Try insert, ignore conflict
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to insert tag: {e}")))?;

        // Fetch the tag_id
        let tag_id: i64 = sqlx::query_scalar("SELECT tag_id FROM tags WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch tag: {e}")))?;

        Ok(tag_id)
    }

    pub async fn list_all(&self) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, TagEntity>("SELECT tag_id, name FROM tags ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list tags: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|e| Tag {
                tag_id: e.tag_id,
                name: e.name,
            })
            .collect())
    }

    pub async fn add_to_correction(&self, correction_id: &str, tag_id: i64) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO correction_tags (correction_id, tag_id) VALUES (?, ?)")
            .bind(correction_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to add tag to correction: {e}"))
            })?;

        Ok(())
    }

    pub async fn remove_from_correction(&self, correction_id: &str, tag_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM correction_tags WHERE correction_id = ? AND tag_id = ?")
            .bind(correction_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to remove tag from correction: {e}"))
            })?;

        Ok(())
    }

    pub async fn list_for_correction(&self, correction_id: &str) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, TagEntity>(
            "SELECT t.tag_id, t.name FROM tags t \
             INNER JOIN correction_tags ct ON t.tag_id = ct.tag_id \
             WHERE ct.correction_id = ? ORDER BY t.name",
        )
        .bind(correction_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list correction tags: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|e| Tag {
                tag_id: e.tag_id,
                name: e.name,
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct TagEntity {
    tag_id: i64,
    name: String,
}

// ============================================================================
// Dataset Repository
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dataset {
    pub dataset_id: String,
    pub name: String,
    pub dataset_type: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetInput {
    pub dataset_id: String,
    pub name: String,
    pub dataset_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetItem {
    pub item_id: String,
    pub dataset_id: String,
    pub prompt: String,
    pub expected_output: Option<String>,
    pub metadata_json: Option<String>,
    pub source_correction_id: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetItemInput {
    pub item_id: String,
    pub dataset_id: String,
    pub prompt: String,
    pub expected_output: Option<String>,
    pub metadata_json: Option<String>,
    pub source_correction_id: Option<String>,
}

pub struct DatasetRepository {
    pool: SqlitePool,
}

impl DatasetRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, dataset: &DatasetInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO datasets (dataset_id, name, type, description) VALUES (?, ?, ?, ?)",
        )
        .bind(&dataset.dataset_id)
        .bind(&dataset.name)
        .bind(&dataset.dataset_type)
        .bind(&dataset.description)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert dataset: {e}")))?;

        Ok(())
    }

    pub async fn get(&self, dataset_id: &str) -> Result<Dataset> {
        let row = sqlx::query_as::<_, DatasetEntity>(
            "SELECT dataset_id, name, type, description, created_at FROM datasets WHERE dataset_id = ?",
        )
        .bind(dataset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch dataset: {e}")))?;

        match row {
            Some(entity) => Ok(entity.into()),
            None => Err(AppError::NotFound(format!(
                "Dataset not found: {}",
                dataset_id
            ))),
        }
    }

    pub async fn list_by_type(&self, dataset_type: &str) -> Result<Vec<Dataset>> {
        let rows = sqlx::query_as::<_, DatasetEntity>(
            "SELECT dataset_id, name, type, description, created_at FROM datasets \
             WHERE type = ? ORDER BY created_at DESC",
        )
        .bind(dataset_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list datasets: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn list_all(&self) -> Result<Vec<Dataset>> {
        let rows = sqlx::query_as::<_, DatasetEntity>(
            "SELECT dataset_id, name, type, description, created_at FROM datasets ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list datasets: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn delete(&self, dataset_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM datasets WHERE dataset_id = ?")
            .bind(dataset_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete dataset: {e}")))?;

        Ok(result.rows_affected())
    }

    pub async fn insert_item(&self, item: &DatasetItemInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO dataset_items (item_id, dataset_id, prompt, expected_output, metadata_json, source_correction_id) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&item.item_id)
        .bind(&item.dataset_id)
        .bind(&item.prompt)
        .bind(&item.expected_output)
        .bind(&item.metadata_json)
        .bind(&item.source_correction_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert dataset item: {e}")))?;

        Ok(())
    }

    pub async fn list_items(&self, dataset_id: &str) -> Result<Vec<DatasetItem>> {
        let rows = sqlx::query_as::<_, DatasetItemEntity>(
            "SELECT item_id, dataset_id, prompt, expected_output, metadata_json, source_correction_id, created_at \
             FROM dataset_items WHERE dataset_id = ? ORDER BY created_at",
        )
        .bind(dataset_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list dataset items: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn count_items(&self, dataset_id: &str) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dataset_items WHERE dataset_id = ?")
                .bind(dataset_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    AppError::DatabaseError(format!("Failed to count dataset items: {e}"))
                })?;

        Ok(count)
    }
}

#[derive(sqlx::FromRow)]
struct DatasetEntity {
    dataset_id: String,
    name: String,
    #[sqlx(rename = "type")]
    dataset_type: String,
    description: Option<String>,
    created_at: String,
}

impl From<DatasetEntity> for Dataset {
    fn from(entity: DatasetEntity) -> Self {
        Self {
            dataset_id: entity.dataset_id,
            name: entity.name,
            dataset_type: entity.dataset_type,
            description: entity.description,
            created_at: Some(entity.created_at),
        }
    }
}

#[derive(sqlx::FromRow)]
struct DatasetItemEntity {
    item_id: String,
    dataset_id: String,
    prompt: String,
    expected_output: Option<String>,
    metadata_json: Option<String>,
    source_correction_id: Option<String>,
    created_at: String,
}

impl From<DatasetItemEntity> for DatasetItem {
    fn from(entity: DatasetItemEntity) -> Self {
        Self {
            item_id: entity.item_id,
            dataset_id: entity.dataset_id,
            prompt: entity.prompt,
            expected_output: entity.expected_output,
            metadata_json: entity.metadata_json,
            source_correction_id: entity.source_correction_id,
            created_at: Some(entity.created_at),
        }
    }
}

// ============================================================================
// Run Datasets Repository
// ============================================================================

pub struct RunDatasetsRepository {
    pool: SqlitePool,
}

impl RunDatasetsRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn add(
        &self,
        run_id: &str,
        dataset_id: &str,
        split: &str,
        weight: f64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_datasets (run_id, dataset_id, split, weight) VALUES (?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(dataset_id)
        .bind(split)
        .bind(weight)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to attach dataset to run: {e}")))?;

        Ok(())
    }

    pub async fn list_for_run(&self, run_id: &str) -> Result<Vec<(String, String, f64)>> {
        let rows = sqlx::query_as::<_, RunDatasetEntity>(
            "SELECT dataset_id, split, weight FROM run_datasets WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list run datasets: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|r| (r.dataset_id, r.split, r.weight))
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct RunDatasetEntity {
    dataset_id: String,
    split: String,
    weight: f64,
}

// ============================================================================
// Model Repository
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub model_id: String,
    pub display_name: String,
    pub provider: String,
    pub model_family: Option<String>,
    pub default_artifact_path: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInput {
    pub model_id: String,
    pub display_name: String,
    pub provider: String,
    pub model_family: Option<String>,
    pub default_artifact_path: Option<String>,
}

pub struct ModelRepository {
    pool: SqlitePool,
}

impl ModelRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, model: &ModelInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO models (model_id, display_name, provider, model_family, default_artifact_path) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&model.model_id)
        .bind(&model.display_name)
        .bind(&model.provider)
        .bind(&model.model_family)
        .bind(&model.default_artifact_path)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert model: {e}")))?;

        Ok(())
    }

    pub async fn get(&self, model_id: &str) -> Result<Model> {
        let row = sqlx::query_as::<_, ModelEntity>(
            "SELECT model_id, display_name, provider, model_family, default_artifact_path, created_at \
             FROM models WHERE model_id = ?",
        )
        .bind(model_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch model: {e}")))?;

        match row {
            Some(entity) => Ok(entity.into()),
            None => Err(AppError::NotFound(format!("Model not found: {}", model_id))),
        }
    }

    pub async fn list_all(&self) -> Result<Vec<Model>> {
        let rows = sqlx::query_as::<_, ModelEntity>(
            "SELECT model_id, display_name, provider, model_family, default_artifact_path, created_at \
             FROM models ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list models: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn list_by_provider(&self, provider: &str) -> Result<Vec<Model>> {
        let rows = sqlx::query_as::<_, ModelEntity>(
            "SELECT model_id, display_name, provider, model_family, default_artifact_path, created_at \
             FROM models WHERE provider = ? ORDER BY created_at DESC",
        )
        .bind(provider)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list models: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn delete(&self, model_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM models WHERE model_id = ?")
            .bind(model_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete model: {e}")))?;

        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct ModelEntity {
    model_id: String,
    display_name: String,
    provider: String,
    model_family: Option<String>,
    default_artifact_path: Option<String>,
    created_at: String,
}

impl From<ModelEntity> for Model {
    fn from(entity: ModelEntity) -> Self {
        Self {
            model_id: entity.model_id,
            display_name: entity.display_name,
            provider: entity.provider,
            model_family: entity.model_family,
            default_artifact_path: entity.default_artifact_path,
            created_at: Some(entity.created_at),
        }
    }
}

// ============================================================================
// Run Artifacts Repository
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunArtifact {
    pub artifact_id: String,
    pub run_id: String,
    pub kind: String,
    pub path: String,
    pub hash: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunArtifactInput {
    pub artifact_id: String,
    pub run_id: String,
    pub kind: String,
    pub path: String,
    pub hash: Option<String>,
    pub size_bytes: Option<i64>,
}

pub struct RunArtifactsRepository {
    pool: SqlitePool,
}

impl RunArtifactsRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, artifact: &RunArtifactInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_artifacts (artifact_id, run_id, kind, path, hash, size_bytes) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&artifact.artifact_id)
        .bind(&artifact.run_id)
        .bind(&artifact.kind)
        .bind(&artifact.path)
        .bind(&artifact.hash)
        .bind(artifact.size_bytes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert run artifact: {e}")))?;

        Ok(())
    }

    pub async fn list_for_run(&self, run_id: &str) -> Result<Vec<RunArtifact>> {
        let rows = sqlx::query_as::<_, RunArtifactEntity>(
            "SELECT artifact_id, run_id, kind, path, hash, size_bytes, created_at \
             FROM run_artifacts WHERE run_id = ? ORDER BY created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list run artifacts: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn list_by_kind(&self, run_id: &str, kind: &str) -> Result<Vec<RunArtifact>> {
        let rows = sqlx::query_as::<_, RunArtifactEntity>(
            "SELECT artifact_id, run_id, kind, path, hash, size_bytes, created_at \
             FROM run_artifacts WHERE run_id = ? AND kind = ? ORDER BY created_at",
        )
        .bind(run_id)
        .bind(kind)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to list run artifacts by kind: {e}"))
        })?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn delete(&self, artifact_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM run_artifacts WHERE artifact_id = ?")
            .bind(artifact_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete run artifact: {e}")))?;

        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct RunArtifactEntity {
    artifact_id: String,
    run_id: String,
    kind: String,
    path: String,
    hash: Option<String>,
    size_bytes: Option<i64>,
    created_at: String,
}

impl From<RunArtifactEntity> for RunArtifact {
    fn from(entity: RunArtifactEntity) -> Self {
        Self {
            artifact_id: entity.artifact_id,
            run_id: entity.run_id,
            kind: entity.kind,
            path: entity.path,
            hash: entity.hash,
            size_bytes: entity.size_bytes,
            created_at: Some(entity.created_at),
        }
    }
}

// ============================================================================
// Soft Labels Repository (Knowledge Distillation)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabel {
    pub soft_label_id: String,
    pub prompt: String,
    pub prompt_hash: String,
    pub teacher_model_id: String,
    pub teacher_output: String,
    pub soft_label_type: String, // "logits", "one_hot", "text_only"
    pub temperature: f64,
    pub metadata_json: Option<String>,
    pub created_at: Option<String>,
    // Note: soft_labels_blob is handled separately for binary data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabelInput {
    pub soft_label_id: String,
    pub prompt: String,
    pub prompt_hash: String,
    pub teacher_model_id: String,
    pub teacher_output: String,
    pub soft_label_type: String,
    pub soft_labels_blob: Option<Vec<u8>>, // Float32 array [seq_len, vocab_size]
    pub temperature: f64,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabelGenerationInput {
    pub prompts: Vec<String>,
    pub teacher_model_id: String,
    pub temperature: f64,
    pub soft_label_type: String, // "logits", "one_hot", "text_only"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabelGenerationResult {
    pub soft_label_ids: Vec<String>,
    pub cached_count: usize,
    pub generated_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>, // Error messages for any failed prompts
}

pub struct SoftLabelRepository {
    pool: SqlitePool,
}

impl SoftLabelRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Compute SHA256 hash of a prompt for deduplication
    fn compute_prompt_hash(prompt: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Insert a new soft label
    pub async fn insert(&self, input: &SoftLabelInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO soft_labels (soft_label_id, prompt, prompt_hash, teacher_model_id, \
             teacher_output, soft_label_type, soft_labels_blob, temperature, metadata_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&input.soft_label_id)
        .bind(&input.prompt)
        .bind(&input.prompt_hash)
        .bind(&input.teacher_model_id)
        .bind(&input.teacher_output)
        .bind(&input.soft_label_type)
        .bind(&input.soft_labels_blob)
        .bind(input.temperature)
        .bind(&input.metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert soft label: {e}")))?;

        Ok(())
    }

    /// Get soft label by ID
    pub async fn get(&self, soft_label_id: &str) -> Result<SoftLabel> {
        let row = sqlx::query_as::<_, SoftLabelEntity>(
            "SELECT soft_label_id, prompt, prompt_hash, teacher_model_id, teacher_output, \
             soft_label_type, temperature, metadata_json, created_at \
             FROM soft_labels WHERE soft_label_id = ?",
        )
        .bind(soft_label_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get soft label: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Soft label not found: {soft_label_id}")))?;

        Ok(row.into())
    }

    /// Get soft label by prompt hash and teacher model
    pub async fn get_by_prompt_and_teacher(
        &self,
        prompt_hash: &str,
        teacher_model_id: &str,
    ) -> Result<Option<SoftLabel>> {
        let row = sqlx::query_as::<_, SoftLabelEntity>(
            "SELECT soft_label_id, prompt, prompt_hash, teacher_model_id, teacher_output, \
             soft_label_type, temperature, metadata_json, created_at \
             FROM soft_labels WHERE prompt_hash = ? AND teacher_model_id = ?",
        )
        .bind(prompt_hash)
        .bind(teacher_model_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get soft label by prompt: {e}")))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get soft labels for a training run
    pub async fn list_for_run(&self, run_id: &str) -> Result<Vec<SoftLabel>> {
        let rows = sqlx::query_as::<_, SoftLabelEntity>(
            "SELECT sl.soft_label_id, sl.prompt, sl.prompt_hash, sl.teacher_model_id, sl.teacher_output, \
             sl.soft_label_type, sl.temperature, sl.metadata_json, sl.created_at \
             FROM soft_labels sl \
             JOIN run_soft_labels rsl ON sl.soft_label_id = rsl.soft_label_id \
             WHERE rsl.run_id = ? \
             ORDER BY sl.created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list soft labels for run: {e}")))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get soft label blob (binary data) by ID
    pub async fn get_blob(&self, soft_label_id: &str) -> Result<Option<Vec<u8>>> {
        let blob = sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT soft_labels_blob FROM soft_labels WHERE soft_label_id = ?",
        )
        .bind(soft_label_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get soft label blob: {e}")))?;

        Ok(blob.flatten())
    }

    /// Delete soft label by ID
    pub async fn delete(&self, soft_label_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM soft_labels WHERE soft_label_id = ?")
            .bind(soft_label_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete soft label: {e}")))?;

        Ok(result.rows_affected())
    }

    /// Link soft label to correction
    pub async fn link_to_correction(&self, correction_id: &str, soft_label_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO correction_soft_labels (correction_id, soft_label_id) VALUES (?, ?) \
             ON CONFLICT(correction_id, soft_label_id) DO NOTHING",
        )
        .bind(correction_id)
        .bind(soft_label_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to link soft label to correction: {e}"))
        })?;

        Ok(())
    }

    /// Link soft label to training run
    pub async fn link_to_run(&self, run_id: &str, soft_label_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_soft_labels (run_id, soft_label_id) VALUES (?, ?) \
             ON CONFLICT(run_id, soft_label_id) DO NOTHING",
        )
        .bind(run_id)
        .bind(soft_label_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to link soft label to run: {e}")))?;

        Ok(())
    }

    /// Export soft labels to a JSONL file for training
    pub async fn export_for_training(
        &self,
        run_id: &str,
        output_path: &std::path::Path,
    ) -> Result<usize> {
        use std::io::Write;

        let soft_labels = self.list_for_run(run_id).await?;
        let mut file = std::fs::File::create(output_path)
            .map_err(|e| AppError::IoError(format!("Failed to create soft labels file: {e}")))?;

        for sl in &soft_labels {
            // Get the blob if available
            let blob_base64 = self
                .get_blob(&sl.soft_label_id)
                .await?
                .map(|b| base64::prelude::BASE64_STANDARD.encode(&b));

            let record = serde_json::json!({
                "soft_label_id": sl.soft_label_id,
                "prompt": sl.prompt,
                "teacher_output": sl.teacher_output,
                "soft_label_type": sl.soft_label_type,
                "soft_labels_blob_base64": blob_base64,
                "temperature": sl.temperature,
            });

            writeln!(
                file,
                "{}",
                serde_json::to_string(&record).map_err(|e| AppError::Internal(format!(
                    "Failed to serialize soft label: {e}"
                )))?
            )
            .map_err(|e| AppError::IoError(format!("Failed to write soft label: {e}")))?;
        }

        Ok(soft_labels.len())
    }
}

#[derive(sqlx::FromRow)]
pub struct SoftLabelEntity {
    soft_label_id: String,
    prompt: String,
    prompt_hash: String,
    teacher_model_id: String,
    teacher_output: String,
    soft_label_type: String,
    temperature: f64,
    metadata_json: Option<String>,
    created_at: String,
}

impl From<SoftLabelEntity> for SoftLabel {
    fn from(entity: SoftLabelEntity) -> Self {
        Self {
            soft_label_id: entity.soft_label_id,
            prompt: entity.prompt,
            prompt_hash: entity.prompt_hash,
            teacher_model_id: entity.teacher_model_id,
            teacher_output: entity.teacher_output,
            soft_label_type: entity.soft_label_type,
            temperature: entity.temperature,
            metadata_json: entity.metadata_json,
            created_at: Some(entity.created_at),
        }
    }
}
