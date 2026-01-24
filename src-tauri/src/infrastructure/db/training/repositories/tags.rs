use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub tag_id: i64,
    pub name: String,
}

pub struct TagRepository {
    pool: SqlitePool,
}

impl TagRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn get_or_create(&self, name: &str) -> Result<i64> {
        // Try insert, ignore conflict
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to insert tag: {e}")))?;

        // Fetch the tag_id
        let tag_id: i64 = sqlx::query_scalar("SELECT tag_id FROM tags WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch tag: {e}")))?;

        Ok(tag_id)
    }

    pub async fn list_all(&self) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, TagEntity>("SELECT tag_id, name FROM tags ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list tags: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|e| Tag {
                tag_id: e.tag_id,
                name: e.name,
            })
            .collect())
    }

    pub async fn add_to_correction(&self, correction_id: &str, tag_id: i64) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO correction_tags (correction_id, tag_id) VALUES (?, ?)")
            .bind(correction_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to add tag to correction: {e}"))
            })?;

        Ok(())
    }

    pub async fn remove_from_correction(&self, correction_id: &str, tag_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM correction_tags WHERE correction_id = ? AND tag_id = ?")
            .bind(correction_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to remove tag from correction: {e}"))
            })?;

        Ok(())
    }

    pub async fn list_for_correction(&self, correction_id: &str) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, TagEntity>(
            "SELECT t.tag_id, t.name FROM tags t \
             INNER JOIN correction_tags ct ON t.tag_id = ct.tag_id \
             WHERE ct.correction_id = ? ORDER BY t.name",
        )
        .bind(correction_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list correction tags: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|e| Tag {
                tag_id: e.tag_id,
                name: e.name,
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct TagEntity {
    tag_id: i64,
    name: String,
}
