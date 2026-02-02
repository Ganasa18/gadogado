//! Context settings repository for RAG context management
//! Handles global RAG settings and model context limits

use crate::domain::context_config::{ContextWindowConfig, ModelContextLimit};
use crate::domain::error::{AppError, Result};
use super::RagRepository;
use sqlx::Row;

impl RagRepository {
    /// Get global RAG context settings
    pub async fn get_global_settings(&self) -> Result<ContextWindowConfig> {
        let row = sqlx::query_as::<_, GlobalSettingsRow>(
            "SELECT max_context_tokens, max_history_messages, enable_compaction,
                   compaction_strategy, summary_threshold, reserved_for_response,
                   small_model_threshold, large_model_threshold
            FROM rag_global_settings WHERE id = 1"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get global settings: {}", e)))?;

        let compaction_strategy = row.compaction_strategy.parse()
            .unwrap_or(crate::domain::context_config::CompactionStrategy::Adaptive);

        Ok(ContextWindowConfig {
            max_context_tokens: row.max_context_tokens as usize,
            max_history_messages: row.max_history_messages as usize,
            enable_compaction: row.enable_compaction != 0,
            compaction_strategy,
            summary_threshold: row.summary_threshold as usize,
            reserved_for_response: row.reserved_for_response as usize,
            small_model_threshold: row.small_model_threshold as usize,
            large_model_threshold: row.large_model_threshold as usize,
        })
    }

    /// Update global RAG context settings
    pub async fn update_global_settings(&self, settings: &ContextWindowConfig) -> Result<()> {
        sqlx::query(
            "UPDATE rag_global_settings
             SET max_context_tokens = ?,
                 max_history_messages = ?,
                 enable_compaction = ?,
                 compaction_strategy = ?,
                 summary_threshold = ?,
                 reserved_for_response = ?,
                 small_model_threshold = ?,
                 large_model_threshold = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = 1"
        )
        .bind(settings.max_context_tokens as i64)
        .bind(settings.max_history_messages as i64)
        .bind(settings.enable_compaction as i32)
        .bind(settings.compaction_strategy.to_string())
        .bind(settings.summary_threshold as i64)
        .bind(settings.reserved_for_response as i64)
        .bind(settings.small_model_threshold as i64)
        .bind(settings.large_model_threshold as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update global settings: {}", e)))?;

        Ok(())
    }

    /// Get model context limit for specific provider/model
    pub async fn get_model_limit(
        &self,
        provider: &str,
        model_name: &str,
    ) -> Result<Option<ModelContextLimit>> {
        let result = sqlx::query_as::<_, ModelContextLimitRow>(
            "SELECT id, provider, model_name, context_window, max_output_tokens
             FROM model_context_limits
             WHERE provider = ? AND model_name = ?"
        )
        .bind(provider)
        .bind(model_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get model limit: {}", e)))?;

        Ok(result.map(|row| ModelContextLimit {
            id: row.id,
            provider: row.provider,
            model_name: row.model_name,
            context_window: row.context_window as usize,
            max_output_tokens: row.max_output_tokens as usize,
        }))
    }

    /// Get or infer model limit with fallback
    pub async fn get_or_infer_limit(
        &self,
        provider: &str,
        model_name: &str,
    ) -> Result<ModelContextLimit> {
        // Try exact match first
        if let Some(limit) = self.get_model_limit(provider, model_name).await? {
            return Ok(limit);
        }

        // Try provider default (model_name = 'default')
        if let Some(limit) = self.get_model_limit(provider, "default").await? {
            return Ok(ModelContextLimit {
                id: limit.id,
                provider: limit.provider,
                model_name: model_name.to_string(),
                context_window: limit.context_window,
                max_output_tokens: limit.max_output_tokens,
            });
        }

        // Fallback to provider defaults
        self.get_provider_default(provider, model_name).await
    }

    /// Get default limit for a provider (when not in database)
    async fn get_provider_default(
        &self,
        provider: &str,
        model_name: &str,
    ) -> Result<ModelContextLimit> {
        let (context_window, max_output_tokens) = match provider {
            "local" | "llama_cpp" => (4096, 1024),
            "ollama" => (8192, 2048),
            "openai" | "openrouter" => (128000, 16384),
            "gemini" => (1000000, 8192),
            "cli_proxy" => (128000, 4096),
            _ => (4096, 1024), // Conservative default
        };

        Ok(ModelContextLimit {
            id: 0, // 0 means not from database
            provider: provider.to_string(),
            model_name: model_name.to_string(),
            context_window,
            max_output_tokens,
        })
    }

    /// Get all model limits from database
    pub async fn get_all_model_limits(&self) -> Result<Vec<ModelContextLimit>> {
        let limits = sqlx::query_as::<_, ModelContextLimitRow>(
            "SELECT id, provider, model_name, context_window, max_output_tokens
             FROM model_context_limits
             ORDER BY provider, model_name"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get all model limits: {}", e)))?;

        Ok(limits.into_iter().map(|row| ModelContextLimit {
            id: row.id,
            provider: row.provider,
            model_name: row.model_name,
            context_window: row.context_window as usize,
            max_output_tokens: row.max_output_tokens as usize,
        }).collect())
    }

    /// Insert or update model context limit
    pub async fn upsert_model_limit(
        &self,
        provider: &str,
        model_name: &str,
        context_window: usize,
        max_output_tokens: usize,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO model_context_limits (provider, model_name, context_window, max_output_tokens)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(provider, model_name) DO UPDATE SET
             context_window = excluded.context_window,
             max_output_tokens = excluded.max_output_tokens
             RETURNING id"
        )
        .bind(provider)
        .bind(model_name)
        .bind(context_window as i64)
        .bind(max_output_tokens as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to upsert model limit: {}", e)))?;

        let id: i64 = result.try_get::<i64, _>("id")
            .map_err(|e| AppError::Internal(format!("Failed to get id: {}", e)))?;

        Ok(id)
    }
}

/// Internal entity for global settings query
#[derive(sqlx::FromRow)]
struct GlobalSettingsRow {
    max_context_tokens: i64,
    max_history_messages: i64,
    enable_compaction: i32,
    compaction_strategy: String,
    summary_threshold: i64,
    reserved_for_response: i64,
    small_model_threshold: i64,
    large_model_threshold: i64,
}

/// Internal entity for model context limit query
#[derive(sqlx::FromRow)]
struct ModelContextLimitRow {
    id: i64,
    provider: String,
    model_name: String,
    context_window: i64,
    max_output_tokens: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a test database setup
    // For now, they serve as documentation of expected behavior
}
