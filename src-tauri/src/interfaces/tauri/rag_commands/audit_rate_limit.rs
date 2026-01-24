use crate::application::use_cases::allowlist_validator::AllowlistValidator;
use crate::application::use_cases::audit_service::{AuditLogEntry, AuditService};
use crate::application::use_cases::chunking::{ChunkConfig, ChunkEngine, ChunkStrategy};
use crate::application::use_cases::data_protection::{ExternalLlmPolicy, LlmRoute};
use crate::application::use_cases::prompt_engine::{PromptEngine, VerificationResult};
use crate::application::use_cases::rag_analytics::{AnalyticsEvent, AnalyticsSummary};
use crate::application::use_cases::rag_config::{
    CacheConfig, ChatConfig, ChunkingConfig, ConfigValidation, EmbeddingConfig, FeedbackRating,
    FeedbackStats, OcrConfig, RagConfig, RetrievalConfig, UserFeedback,
};
use crate::application::use_cases::rag_ingestion::OcrResult;
use crate::application::use_cases::rag_validation::{
    RagValidationSuite, ValidationCase, ValidationOptions, ValidationReport,
};
use crate::application::use_cases::rate_limiter::{RateLimitResult, RateLimitStatus, RateLimiter};
use crate::application::use_cases::sql_compiler::{DbType, SqlCompiler};
use crate::application::use_cases::sql_rag_router::SqlRagRouter;
use crate::domain::error::Result;
use crate::domain::rag_entities::{
    DbAllowlistProfile, DbConnection, DbConnectionInput, RagCollection, RagCollectionInput,
    RagDocument, RagDocumentChunk, RagExcelData,
};
use crate::interfaces::http::add_log;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;


use super::types::*;

#[tauri::command]
pub async fn db_get_audit_recent(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    limit: Option<i32>,
) -> std::result::Result<Vec<crate::application::use_cases::audit_service::AuditLogRecord>, String>
{
    let limit = limit.unwrap_or(50).clamp(1, 500);

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Fetching audit logs for collection {}, limit={}",
            collection_id, limit
        ),
    );

    match state
        .audit_service
        .get_recent_audit_logs(collection_id, limit)
        .await
    {
        Ok(logs) => Ok(logs),
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to fetch audit logs: {}", e),
            );
            Err(e.to_string())
        }
    }
}

/// Get current rate limit status for a collection

#[tauri::command]
pub async fn db_get_rate_limit_status(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> std::result::Result<RateLimitStatus, String> {
    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Fetching rate limit status for collection {}",
            collection_id
        ),
    );

    match state.rate_limiter.get_status(collection_id).await {
        Ok(status) => Ok(status),
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to fetch rate limit status: {}", e),
            );
            Err(e.to_string())
        }
    }
}

