use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
