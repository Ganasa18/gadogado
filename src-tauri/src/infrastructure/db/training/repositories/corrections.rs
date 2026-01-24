use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    pub correction_id: String,
    pub prompt: String,
    pub student_output: String,
    pub corrected_output: String,
    pub accuracy_rating: i64,
    pub relevance_rating: Option<i64>,
    pub safety_rating: Option<i64>,
    pub domain_notes: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionInput {
    pub correction_id: String,
    pub prompt: String,
    pub student_output: String,
    pub corrected_output: String,
    pub accuracy_rating: i64,
    pub relevance_rating: Option<i64>,
    pub safety_rating: Option<i64>,
    pub domain_notes: Option<String>,
}

pub struct CorrectionRepository {
    pool: SqlitePool,
}

impl CorrectionRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn insert(&self, correction: &CorrectionInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO corrections (correction_id, prompt, student_output, corrected_output, accuracy_rating, relevance_rating, safety_rating, domain_notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&correction.correction_id)
        .bind(&correction.prompt)
        .bind(&correction.student_output)
        .bind(&correction.corrected_output)
        .bind(correction.accuracy_rating)
        .bind(correction.relevance_rating)
        .bind(correction.safety_rating)
        .bind(&correction.domain_notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert correction: {e}")))?;
        Ok(())
    }

    pub async fn get(&self, correction_id: &str) -> Result<Correction> {
        let row = sqlx::query_as::<_, CorrectionEntity>(
            "SELECT correction_id, prompt, student_output, corrected_output, accuracy_rating, relevance_rating, safety_rating, domain_notes, created_at \
             FROM corrections WHERE correction_id = ?",
        )
        .bind(correction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch correction: {e}")))?;

        match row {
            Some(entity) => Ok(entity.into()),
            None => Err(AppError::NotFound(format!(
                "Correction not found: {}",
                correction_id
            ))),
        }
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<Correction>> {
        let rows = sqlx::query_as::<_, CorrectionEntity>(
            "SELECT correction_id, prompt, student_output, corrected_output, accuracy_rating, relevance_rating, safety_rating, domain_notes, created_at \
             FROM corrections ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list corrections: {e}")))?;

        Ok(rows.into_iter().map(|e| e.into()).collect())
    }

    pub async fn delete(&self, correction_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM corrections WHERE correction_id = ?")
            .bind(correction_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete correction: {e}")))?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct CorrectionEntity {
    correction_id: String,
    prompt: String,
    student_output: String,
    corrected_output: String,
    accuracy_rating: i64,
    relevance_rating: Option<i64>,
    safety_rating: Option<i64>,
    domain_notes: Option<String>,
    created_at: String,
}

impl From<CorrectionEntity> for Correction {
    fn from(entity: CorrectionEntity) -> Self {
        Self {
            correction_id: entity.correction_id,
            prompt: entity.prompt,
            student_output: entity.student_output,
            corrected_output: entity.corrected_output,
            accuracy_rating: entity.accuracy_rating,
            relevance_rating: entity.relevance_rating,
            safety_rating: entity.safety_rating,
            domain_notes: entity.domain_notes,
            created_at: Some(entity.created_at),
        }
    }
}
