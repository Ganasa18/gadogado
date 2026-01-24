use crate::domain::error::{AppError, Result};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
