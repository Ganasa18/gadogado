use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to list previous model versions: {e}"))
        })?;

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
