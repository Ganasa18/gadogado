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

use crate::application::use_cases::conversation_service::{Conversation, ConversationMessage};

#[tauri::command]
pub async fn rag_create_conversation(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
    title: Option<String>,
) -> Result<i64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Creating conversation for collection: {:?}", collection_id),
    );

    state
        .conversation_service
        .create_conversation(collection_id, title.as_deref())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to create conversation: {}", e),
            );
            e
        })
}

/// Add a message to a conversation

#[tauri::command]
pub async fn rag_add_conversation_message(
    state: State<'_, Arc<super::AppState>>,
    conversation_id: i64,
    role: String,
    content: String,
    sources: Option<Vec<i64>>,
) -> Result<i64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Adding {} message to conversation {}",
            role, conversation_id
        ),
    );

    // Convert sources to JSON string if provided
    let sources_json = sources.map(|s| serde_json::to_string(&s).unwrap_or_default());

    state
        .conversation_service
        .add_message(conversation_id, &role, &content, sources_json.as_deref())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to add message: {}", e),
            );
            e
        })
}

/// Get messages for a conversation

#[tauri::command]
pub async fn rag_get_conversation_messages(
    state: State<'_, Arc<super::AppState>>,
    conversation_id: i64,
    limit: Option<i64>,
) -> Result<Vec<ConversationMessage>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting messages for conversation {}", conversation_id),
    );

    state
        .conversation_service
        .get_messages(conversation_id, limit.unwrap_or(100))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get messages: {}", e),
            );
            e
        })
}

/// List conversations for a collection (or all if no collection_id)

#[tauri::command]
pub async fn rag_list_conversations(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
) -> Result<Vec<Conversation>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Listing conversations for collection: {:?}", collection_id),
    );

    state
        .conversation_service
        .list_conversations(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list conversations: {}", e),
            );
            e
        })
}

/// Delete a conversation and all its messages

#[tauri::command]
pub async fn rag_delete_conversation(
    state: State<'_, Arc<super::AppState>>,
    conversation_id: i64,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting conversation {}", conversation_id),
    );

    state
        .conversation_service
        .delete_conversation(conversation_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to delete conversation: {}", e),
            );
            e
        })
}

// ============================================================
// QUALITY ANALYTICS API
// ============================================================

use crate::domain::rag_entities::{
    CollectionQualityMetrics, DocumentWarning, DocumentWarningInput, RetrievalGap,
    RetrievalGapInput,
};

// Get collection quality metrics

