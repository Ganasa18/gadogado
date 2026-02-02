use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationMetricInput {
    pub version_id: String,
    pub dataset_id: String,
    pub metric_name: String,
    pub metric_value: f64,
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
