use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingMethod {
    FineTune,
    KnowledgeDistillation,
    Hybrid,
}

impl TrainingMethod {
    pub(super) fn as_db(&self) -> &'static str {
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
    pub(super) fn as_db(&self) -> &'static str {
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
            None => Err(AppError::NotFound(format!("Training run not found: {}", run_id))),
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
