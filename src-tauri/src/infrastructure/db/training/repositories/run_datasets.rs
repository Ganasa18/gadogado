use crate::domain::error::{AppError, Result};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
        sqlx::query("INSERT INTO run_datasets (run_id, dataset_id, split, weight) VALUES (?, ?, ?, ?)")
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
