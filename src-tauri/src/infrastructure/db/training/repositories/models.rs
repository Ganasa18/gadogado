use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
