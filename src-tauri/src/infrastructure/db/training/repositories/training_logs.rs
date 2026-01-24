use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
