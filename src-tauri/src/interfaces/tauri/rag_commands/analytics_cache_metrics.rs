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
pub async fn rag_get_analytics_summary(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
) -> Result<AnalyticsSummary> {
    add_log(&state.logs, "INFO", "RAG", "Fetching analytics summary");
    if let Some(cid) = collection_id {
        Ok(state.analytics_logger.summary_for_collection(Some(cid)))
    } else {
        Ok(state.analytics_logger.summary())
    }
}


#[tauri::command]
pub async fn rag_get_recent_analytics(
    state: State<'_, Arc<super::AppState>>,
    limit: Option<usize>,
    collection_id: Option<i64>,
) -> Result<Vec<AnalyticsEvent>> {
    let limit = limit.unwrap_or(50);
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Fetching {} analytics events", limit),
    );
    Ok(state
        .analytics_logger
        .recent_events_by_collection(limit, collection_id))
}


#[tauri::command]
pub async fn rag_clear_analytics(state: State<'_, Arc<super::AppState>>) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Clearing analytics events");
    state.analytics_logger.clear();
    Ok("Analytics cleared successfully".to_string())
}

#[tauri::command]
pub async fn rag_health_check(state: State<'_, Arc<super::AppState>>) -> Result<HealthReport> {
    add_log(&state.logs, "INFO", "RAG", "Running health check");

    // Check database
    let db_health = match state.rag_repository.list_collections(1).await {
        Ok(_) => ComponentHealth {
            status: "healthy".to_string(),
            message: None,
        },
        Err(e) => ComponentHealth {
            status: "unhealthy".to_string(),
            message: Some(format!("Database error: {}", e)),
        },
    };

    // Check embedding service cache
    let cache_stats = state.embedding_service.cache_stats();
    let cache_health = CacheHealth {
        status: "healthy".to_string(),
        total_entries: cache_stats.total_entries,
        valid_entries: cache_stats.valid_entries,
        max_size: cache_stats.max_size,
    };

    // Embedding service is considered healthy if it exists
    let embedding_health = ComponentHealth {
        status: "healthy".to_string(),
        message: Some(format!(
            "Cache: {}/{} entries",
            cache_stats.valid_entries, cache_stats.max_size
        )),
    };

    // Overall status
    let overall_status = if db_health.status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };

    Ok(HealthReport {
        status: overall_status.to_string(),
        components: HealthComponents {
            database: db_health,
            embedding_service: embedding_health,
            embedding_cache: cache_health,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Clear embedding cache

#[tauri::command]
pub async fn rag_clear_cache(state: State<'_, Arc<super::AppState>>) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Clearing embedding cache");

    state.embedding_service.clear_cache();

    Ok("Cache cleared successfully".to_string())
}

// Helper function to truncate messages for summaries
pub(crate) fn truncate_message(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

// ============================================================
// METRICS AND EXPERIMENTS
// ============================================================

use crate::application::use_cases::rag_metrics::{
    AggregatedMetrics, DocumentQualityMetrics, DocumentQualitySummary, ExperimentConfig,
    RagOperationMetrics,
};
use crate::application::use_cases::retrieval_service::RetrievalCacheStats;

/// Get RAG performance metrics for the last N minutes

#[tauri::command]
pub async fn rag_get_metrics(
    state: State<'_, Arc<super::AppState>>,
    minutes: Option<u64>,
) -> Result<AggregatedMetrics> {
    add_log(&state.logs, "INFO", "RAG", "Fetching RAG metrics");

    let minutes = minutes.unwrap_or(60); // Default to last hour
    let metrics = state.metrics_collector.get_aggregated_metrics(minutes);

    Ok(metrics)
}

/// Record a RAG operation metric

#[tauri::command]
pub async fn rag_record_metric(
    state: State<'_, Arc<super::AppState>>,
    metrics: RagOperationMetrics,
) -> Result<()> {
    state.metrics_collector.record_operation(metrics);
    Ok(())
}

/// Record document quality metrics

#[tauri::command]
pub async fn rag_record_document_quality(
    state: State<'_, Arc<super::AppState>>,
    metrics: DocumentQualityMetrics,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Recording quality metrics for document: {}",
            metrics.document_name
        ),
    );

    state.metrics_collector.record_document_quality(metrics);
    Ok(())
}

/// Get document quality summary

#[tauri::command]
pub async fn rag_get_document_quality_summary(
    state: State<'_, Arc<super::AppState>>,
) -> Result<DocumentQualitySummary> {
    let summary = state.metrics_collector.get_document_quality_summary();
    Ok(summary)
}

/// Clear all RAG metrics

#[tauri::command]
pub async fn rag_clear_metrics(state: State<'_, Arc<super::AppState>>) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Clearing RAG metrics");
    state.metrics_collector.clear();
    Ok("Metrics cleared successfully".to_string())
}

/// Get retrieval cache statistics

#[tauri::command]
pub async fn rag_get_retrieval_cache_stats(
    state: State<'_, Arc<super::AppState>>,
) -> Result<RetrievalCacheStats> {
    let stats = state.retrieval_service.cache_stats();
    Ok(stats)
}

/// Clear retrieval cache

#[tauri::command]
pub async fn rag_clear_retrieval_cache(state: State<'_, Arc<super::AppState>>) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Clearing retrieval cache");
    state.retrieval_service.clear_cache();
    Ok("Retrieval cache cleared successfully".to_string())
}

/// Invalidate retrieval cache for a specific collection

#[tauri::command]
pub async fn rag_invalidate_collection_cache(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Invalidating cache for collection: {}", collection_id),
    );
    state
        .retrieval_service
        .invalidate_collection_cache(collection_id);
    Ok(format!(
        "Cache invalidated for collection {}",
        collection_id
    ))
}

/// Register a new A/B experiment

#[tauri::command]
pub async fn rag_register_experiment(
    state: State<'_, Arc<super::AppState>>,
    config: ExperimentConfig,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Registering experiment: {} ({})", config.name, config.id),
    );

    state.experiment_manager.register_experiment(config.clone());
    Ok(format!("Experiment {} registered successfully", config.id))
}

/// List active experiments

#[tauri::command]
pub async fn rag_list_experiments(
    state: State<'_, Arc<super::AppState>>,
) -> Result<Vec<ExperimentConfig>> {
    let experiments = state.experiment_manager.list_active_experiments();
    Ok(experiments)
}

/// Assign a session to an experiment variant

#[tauri::command]
pub async fn rag_assign_experiment_variant(
    state: State<'_, Arc<super::AppState>>,
    session_id: String,
    experiment_id: String,
) -> Result<Option<String>> {
    let variant = state
        .experiment_manager
        .assign_variant(&session_id, &experiment_id);

    if let Some(ref v) = variant {
        add_log(
            &state.logs,
            "INFO",
            "RAG",
            &format!(
                "Assigned session {} to variant {} in experiment {}",
                session_id, v, experiment_id
            ),
        );
    }

    Ok(variant)
}

/// Deactivate an experiment

#[tauri::command]
pub async fn rag_deactivate_experiment(
    state: State<'_, Arc<super::AppState>>,
    experiment_id: String,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deactivating experiment: {}", experiment_id),
    );

    state
        .experiment_manager
        .deactivate_experiment(&experiment_id);
    Ok(format!("Experiment {} deactivated", experiment_id))
}

/// Get system uptime and overall metrics

#[tauri::command]
pub async fn rag_get_system_stats(state: State<'_, Arc<super::AppState>>) -> Result<SystemStats> {
    let metrics = state.metrics_collector.get_aggregated_metrics(60);
    let embedding_cache = state.embedding_service.cache_stats();
    let retrieval_cache = state.retrieval_service.cache_stats();
    let uptime_secs = state.metrics_collector.uptime_secs();

    Ok(SystemStats {
        uptime_secs,
        total_operations: metrics.total_operations,
        avg_latency_ms: metrics.avg_latency_ms,
        cache_hit_rate: metrics.cache_hit_rate,
        embedding_cache_entries: embedding_cache.valid_entries,
        retrieval_cache_entries: retrieval_cache.valid_entries,
        retrieval_cache_hit_rate: retrieval_cache.hit_rate,
    })
}

#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub uptime_secs: u64,
    pub total_operations: usize,
    pub avg_latency_ms: f64,
    pub cache_hit_rate: f32,
    pub embedding_cache_entries: usize,
    pub retrieval_cache_entries: usize,
    pub retrieval_cache_hit_rate: f32,
}

// ============================================================
// DOCUMENT QUALITY
// ============================================================

use crate::application::use_cases::rag_ingestion::DocumentQualityAnalysis;

/// Analyze the quality of an ingested document

#[tauri::command]
pub async fn rag_analyze_document_quality(
    state: State<'_, Arc<super::AppState>>,
    document_id: i64,
) -> Result<DocumentQualityAnalysis> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Analyzing quality for document: {}", document_id),
    );

    let analysis = state
        .rag_ingestion_use_case
        .analyze_document_quality(document_id)
        .await?;

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Quality analysis complete: {} chunks, avg quality: {:.2}",
            analysis.total_chunks, analysis.avg_chunk_quality
        ),
    );

    Ok(analysis)
}

// ============================================================
// CONFIGURATION MANAGEMENT
// ============================================================

// Get current RAG configuration

