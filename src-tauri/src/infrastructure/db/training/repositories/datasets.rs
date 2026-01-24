use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

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
        sqlx::query("INSERT INTO datasets (dataset_id, name, type, description) VALUES (?, ?, ?, ?)")
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
            None => Err(AppError::NotFound(format!("Dataset not found: {}", dataset_id))),
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
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dataset_items WHERE dataset_id = ?")
            .bind(dataset_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to count dataset items: {e}")))?;

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
