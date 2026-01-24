use crate::domain::error::{AppError, Result};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
