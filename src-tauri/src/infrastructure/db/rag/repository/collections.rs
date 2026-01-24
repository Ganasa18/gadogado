use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{RagCollection, RagCollectionInput};

use super::entities::RagCollectionEntity;
use super::RagRepository;

impl RagRepository {
    pub async fn create_collection(&self, input: &RagCollectionInput) -> Result<RagCollection> {
        let result = sqlx::query_as::<_, RagCollectionEntity>(
            "INSERT INTO collections (name, description, kind, config_json) VALUES (?, ?, 'files', '{}') RETURNING *",
        )
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create collection: {}", e)))?;

        Ok(result.into())
    }

    pub async fn create_collection_with_config(
        &self,
        name: &str,
        description: &str,
        kind: &str,
        config_json: &str,
    ) -> Result<RagCollection> {
        let result = sqlx::query_as::<_, RagCollectionEntity>(
            "INSERT INTO collections (name, description, kind, config_json) VALUES (?, ?, ?, ?) RETURNING *",
        )
        .bind(name)
        .bind(description)
        .bind(kind)
        .bind(config_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create collection: {}", e)))?;

        Ok(result.into())
    }

    pub async fn get_collection(&self, id: i64) -> Result<RagCollection> {
        let collection = sqlx::query_as::<_, RagCollectionEntity>(
            "SELECT id, name, description, kind, config_json, created_at FROM collections WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch collection: {}", e)))?;

        match collection {
            Some(collection) => Ok(collection.into()),
            None => Err(AppError::NotFound(format!("Collection not found: {}", id))),
        }
    }

    pub async fn list_collections(&self, limit: i64) -> Result<Vec<RagCollection>> {
        let collections = sqlx::query_as::<_, RagCollectionEntity>(
            "SELECT id, name, description, kind, config_json, created_at FROM collections ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list collections: {}", e)))?;

        Ok(collections.into_iter().map(|c| c.into()).collect())
    }

    pub async fn update_collection_config(&self, id: i64, config_json: &str) -> Result<()> {
        sqlx::query("UPDATE collections SET config_json = ? WHERE id = ?")
            .bind(config_json)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to update collection config: {}", e))
            })?;

        Ok(())
    }

    pub async fn delete_collection(&self, id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete collection: {}", e)))?;

        Ok(result.rows_affected())
    }
}
