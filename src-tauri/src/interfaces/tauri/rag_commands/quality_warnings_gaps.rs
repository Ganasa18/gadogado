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
pub async fn rag_get_document_warnings(
    state: State<'_, Arc<super::AppState>>,
    doc_id: i64,
) -> Result<Vec<DocumentWarning>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting warnings for document {}", doc_id),
    );

    state
        .rag_repository
        .get_document_warnings(doc_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get document warnings: {}", e),
            );
            e
        })
}

/// Create a document warning

#[tauri::command]
pub async fn rag_create_document_warning(
    state: State<'_, Arc<super::AppState>>,
    input: DocumentWarningInput,
) -> Result<DocumentWarning> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Creating warning for document {}: {}",
            input.doc_id, input.warning_type
        ),
    );

    state
        .rag_repository
        .create_warning(&input)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to create warning: {}", e),
            );
            e
        })
}

/// Get low quality documents in a collection

#[tauri::command]
pub async fn rag_get_low_quality_documents(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    threshold: Option<f64>,
    limit: Option<i64>,
) -> Result<Vec<RagDocument>> {
    let threshold = threshold.unwrap_or(0.5);
    let limit = limit.unwrap_or(50);

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Getting low quality documents (threshold: {}) for collection {}",
            threshold, collection_id
        ),
    );

    state
        .rag_repository
        .get_low_quality_documents(collection_id, threshold, limit)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get low quality documents: {}", e),
            );
            e
        })
}

/// Record a retrieval gap for analytics

#[tauri::command]
pub async fn rag_record_retrieval_gap(
    state: State<'_, Arc<super::AppState>>,
    input: RetrievalGapInput,
) -> Result<RetrievalGap> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Recording retrieval gap for collection {}",
            input.collection_id
        ),
    );

    state
        .rag_repository
        .record_retrieval_gap(&input)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to record retrieval gap: {}", e),
            );
            e
        })
}

/// Get retrieval gaps for a collection

#[tauri::command]
pub async fn rag_get_retrieval_gaps(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    limit: Option<i64>,
) -> Result<Vec<RetrievalGap>> {
    let limit = limit.unwrap_or(100);

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting retrieval gaps for collection {}", collection_id),
    );

    state
        .rag_repository
        .get_retrieval_gaps(collection_id, limit)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get retrieval gaps: {}", e),
            );
            e
        })
}
