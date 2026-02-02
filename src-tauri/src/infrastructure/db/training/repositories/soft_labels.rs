use crate::domain::error::{AppError, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::TrainingDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabel {
    pub soft_label_id: String,
    pub prompt: String,
    pub prompt_hash: String,
    pub teacher_model_id: String,
    pub teacher_output: String,
    pub soft_label_type: String, // "logits", "one_hot", "text_only"
    pub temperature: f64,
    pub metadata_json: Option<String>,
    pub created_at: Option<String>,
    // Note: soft_labels_blob is handled separately for binary data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabelInput {
    pub soft_label_id: String,
    pub prompt: String,
    pub prompt_hash: String,
    pub teacher_model_id: String,
    pub teacher_output: String,
    pub soft_label_type: String,
    pub soft_labels_blob: Option<Vec<u8>>, // Float32 array [seq_len, vocab_size]
    pub temperature: f64,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabelGenerationInput {
    pub prompts: Vec<String>,
    pub teacher_model_id: String,
    pub temperature: f64,
    pub soft_label_type: String, // "logits", "one_hot", "text_only"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftLabelGenerationResult {
    pub soft_label_ids: Vec<String>,
    pub cached_count: usize,
    pub generated_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>, // Error messages for any failed prompts
}

pub struct SoftLabelRepository {
    pool: SqlitePool,
}

impl SoftLabelRepository {
    pub fn new(db: &TrainingDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Compute SHA256 hash of a prompt for deduplication
    fn compute_prompt_hash(prompt: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Insert a new soft label
    pub async fn insert(&self, input: &SoftLabelInput) -> Result<()> {
        sqlx::query(
            "INSERT INTO soft_labels (soft_label_id, prompt, prompt_hash, teacher_model_id, \
             teacher_output, soft_label_type, soft_labels_blob, temperature, metadata_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&input.soft_label_id)
        .bind(&input.prompt)
        .bind(&input.prompt_hash)
        .bind(&input.teacher_model_id)
        .bind(&input.teacher_output)
        .bind(&input.soft_label_type)
        .bind(&input.soft_labels_blob)
        .bind(input.temperature)
        .bind(&input.metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert soft label: {e}")))?;

        Ok(())
    }

    /// Get soft label by ID
    pub async fn get(&self, soft_label_id: &str) -> Result<SoftLabel> {
        let row = sqlx::query_as::<_, SoftLabelEntity>(
            "SELECT soft_label_id, prompt, prompt_hash, teacher_model_id, teacher_output, \
             soft_label_type, temperature, metadata_json, created_at \
             FROM soft_labels WHERE soft_label_id = ?",
        )
        .bind(soft_label_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get soft label: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Soft label not found: {soft_label_id}")))?;

        Ok(row.into())
    }

    /// Get soft label by prompt hash and teacher model
    pub async fn get_by_prompt_and_teacher(
        &self,
        prompt_hash: &str,
        teacher_model_id: &str,
    ) -> Result<Option<SoftLabel>> {
        let row = sqlx::query_as::<_, SoftLabelEntity>(
            "SELECT soft_label_id, prompt, prompt_hash, teacher_model_id, teacher_output, \
             soft_label_type, temperature, metadata_json, created_at \
             FROM soft_labels WHERE prompt_hash = ? AND teacher_model_id = ?",
        )
        .bind(prompt_hash)
        .bind(teacher_model_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get soft label by prompt: {e}")))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get soft labels for a training run
    pub async fn list_for_run(&self, run_id: &str) -> Result<Vec<SoftLabel>> {
        let rows = sqlx::query_as::<_, SoftLabelEntity>(
            "SELECT sl.soft_label_id, sl.prompt, sl.prompt_hash, sl.teacher_model_id, sl.teacher_output, \
             sl.soft_label_type, sl.temperature, sl.metadata_json, sl.created_at \
             FROM soft_labels sl \
             JOIN run_soft_labels rsl ON sl.soft_label_id = rsl.soft_label_id \
             WHERE rsl.run_id = ? \
             ORDER BY sl.created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list soft labels for run: {e}")))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get soft label blob (binary data) by ID
    pub async fn get_blob(&self, soft_label_id: &str) -> Result<Option<Vec<u8>>> {
        let blob = sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT soft_labels_blob FROM soft_labels WHERE soft_label_id = ?",
        )
        .bind(soft_label_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get soft label blob: {e}")))?;

        Ok(blob.flatten())
    }

    /// Delete soft label by ID
    pub async fn delete(&self, soft_label_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM soft_labels WHERE soft_label_id = ?")
            .bind(soft_label_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete soft label: {e}")))?;

        Ok(result.rows_affected())
    }

    /// Link soft label to correction
    pub async fn link_to_correction(&self, correction_id: &str, soft_label_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO correction_soft_labels (correction_id, soft_label_id) VALUES (?, ?) \
             ON CONFLICT(correction_id, soft_label_id) DO NOTHING",
        )
        .bind(correction_id)
        .bind(soft_label_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to link soft label to correction: {e}"))
        })?;

        Ok(())
    }

    /// Link soft label to training run
    pub async fn link_to_run(&self, run_id: &str, soft_label_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_soft_labels (run_id, soft_label_id) VALUES (?, ?) \
             ON CONFLICT(run_id, soft_label_id) DO NOTHING",
        )
        .bind(run_id)
        .bind(soft_label_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to link soft label to run: {e}")))?;

        Ok(())
    }

    /// Export soft labels to a JSONL file for training
    pub async fn export_for_training(
        &self,
        run_id: &str,
        output_path: &std::path::Path,
    ) -> Result<usize> {
        use std::io::Write;

        let soft_labels = self.list_for_run(run_id).await?;
        let mut file = std::fs::File::create(output_path)
            .map_err(|e| AppError::IoError(format!("Failed to create soft labels file: {e}")))?;

        for sl in &soft_labels {
            // Get the blob if available
            let blob_base64 = self
                .get_blob(&sl.soft_label_id)
                .await?
                .map(|b| base64::prelude::BASE64_STANDARD.encode(&b));

            let record = serde_json::json!({
                "soft_label_id": sl.soft_label_id,
                "prompt": sl.prompt,
                "teacher_output": sl.teacher_output,
                "soft_label_type": sl.soft_label_type,
                "soft_labels_blob_base64": blob_base64,
                "temperature": sl.temperature,
            });

            writeln!(
                file,
                "{}",
                serde_json::to_string(&record).map_err(|e| {
                    AppError::Internal(format!("Failed to serialize soft label: {e}"))
                })?
            )
            .map_err(|e| AppError::IoError(format!("Failed to write soft label: {e}")))?;
        }

        Ok(soft_labels.len())
    }
}

#[derive(sqlx::FromRow)]
pub struct SoftLabelEntity {
    soft_label_id: String,
    prompt: String,
    prompt_hash: String,
    teacher_model_id: String,
    teacher_output: String,
    soft_label_type: String,
    temperature: f64,
    metadata_json: Option<String>,
    created_at: String,
}

impl From<SoftLabelEntity> for SoftLabel {
    fn from(entity: SoftLabelEntity) -> Self {
        Self {
            soft_label_id: entity.soft_label_id,
            prompt: entity.prompt,
            prompt_hash: entity.prompt_hash,
            teacher_model_id: entity.teacher_model_id,
            teacher_output: entity.teacher_output,
            soft_label_type: entity.soft_label_type,
            temperature: entity.temperature,
            metadata_json: entity.metadata_json,
            created_at: Some(entity.created_at),
        }
    }
}
