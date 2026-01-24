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
use super::chunks::average_score;

#[tauri::command]
pub async fn rag_import_web(
    state: State<'_, Arc<super::AppState>>,
    request: RagWebImportRequest,
) -> Result<RagDocument> {
    let mode_str = match request.mode {
        WebCrawlMode::Html => "HTML",
        WebCrawlMode::Ocr => "OCR (Screenshot + Tesseract)",
    };

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Importing web ({}): {}", mode_str, request.url),
    );

    // CRITICAL SECURITY CHECK: Block web imports for DB collections
    if let Some(coll_id) = request.collection_id {
        let collection = state.rag_repository.get_collection(coll_id).await?;
        if collection.kind == crate::domain::rag_entities::CollectionKind::Db {
            let err_msg = format!(
                "Web import blocked: Collection '{}' (id={}) is a Database Collection. \
                DB Collections are specialized for database queries only and cannot be used with files or web content.",
                collection.name, coll_id
            );
            add_log(&state.logs, "WARN", "RAG", &err_msg);
            return Err(crate::domain::error::AppError::ValidationError(err_msg));
        }
    }

    let start = Instant::now();
    match request.mode {
        WebCrawlMode::Html => {
            // Standard HTML crawl mode
            let result = state
                .rag_ingestion_use_case
                .ingest_web_html(
                    &request.url,
                    request.collection_id,
                    request.max_pages,
                    request.max_depth,
                    state.logs.clone(),
                )
                .await
                .map_err(|e| {
                    add_log(
                        &state.logs,
                        "ERROR",
                        "RAG",
                        &format!("Web import failed: {}", e),
                    );
                    e
                });

            state.analytics_logger.log_extraction(
                "web",
                result.is_ok(),
                start.elapsed().as_millis() as u64,
            );
            result
        }
        WebCrawlMode::Ocr => {
            // OCR mode using Playwright + Tesseract
            let result = state
                .rag_ingestion_use_case
                .ingest_web_ocr(&request.url, request.collection_id, state.logs.clone())
                .await
                .map_err(|e| {
                    add_log(
                        &state.logs,
                        "ERROR",
                        "RAG",
                        &format!("Web OCR import failed: {}", e),
                    );
                    e
                });

            state.analytics_logger.log_extraction(
                "web_ocr",
                result.is_ok(),
                start.elapsed().as_millis() as u64,
            );
            result
        }
    }
}

// ============================================================
// PHASE 6: BACKEND API EXTENSIONS
// ============================================================


#[tauri::command]
pub async fn rag_enhanced_ocr(
    state: State<'_, Arc<super::AppState>>,
    request: EnhancedOcrRequest,
) -> Result<OcrResult> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Running enhanced OCR: {}", request.file_path),
    );

    let start = Instant::now();
    let config = request
        .config
        .unwrap_or_else(|| state.config_manager.get_config().ocr);

    let result = state
        .rag_ingestion_use_case
        .enhanced_ocr(&request.file_path, &config, state.logs.clone())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Enhanced OCR failed: {}", e),
            );
            e
        });

    let doc_type = Path::new(&request.file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown");
    state.analytics_logger.log_extraction(
        doc_type,
        result.is_ok(),
        start.elapsed().as_millis() as u64,
    );

    result
}


#[tauri::command]
pub async fn rag_smart_chunking(
    state: State<'_, Arc<super::AppState>>,
    request: SmartChunkingRequest,
) -> Result<Vec<SmartChunk>> {
    add_log(&state.logs, "INFO", "RAG", "Running smart chunking");

    let config = request
        .config
        .unwrap_or_else(|| state.config_manager.get_config().chunking);

    let strategy = match config.strategy.as_str() {
        "fixed_size" => ChunkStrategy::FixedSize,
        "semantic" => ChunkStrategy::Semantic,
        _ => ChunkStrategy::ContentAware,
    };

    let engine = ChunkEngine::new(ChunkConfig {
        max_chunk_size: config.chunk_size,
        overlap: config.overlap,
        strategy,
        min_chunk_size: 100,
    });

    let chunks = engine.chunk_text(&request.text).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Smart chunking failed: {}", e),
        );
        e
    })?;
    let response = chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| SmartChunk {
            index,
            content: chunk.content,
            token_count: chunk.token_count,
            quality_score: chunk.quality_score,
            content_type: chunk.content_type,
        })
        .collect();

    Ok(response)
}


#[tauri::command]
pub async fn rag_hybrid_retrieval(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    query: String,
    options: Option<HybridRetrievalOptions>,
) -> Result<HybridRetrievalResponse> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Hybrid retrieval in collection {}: {}",
            collection_id, query
        ),
    );

    let start = Instant::now();
    let config = state.config_manager.get_config();
    let top_k = options
        .as_ref()
        .and_then(|o| o.top_k)
        .unwrap_or(config.retrieval.top_k);
    let use_cache = options
        .as_ref()
        .and_then(|o| o.use_cache)
        .unwrap_or(config.cache.enabled);
    let optimized = options.as_ref().and_then(|o| o.optimized).unwrap_or(true);

    if use_cache {
        let (mut results, cache_hit) = state
            .retrieval_service
            .query_cached(collection_id, &query, top_k)
            .await
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "RAG",
                    &format!("Hybrid retrieval (cached) failed: {}", e),
                );
                e
            })?;
        if optimized {
            results = state.retrieval_service.optimize_context(results);
            results.truncate(top_k);
        }

        state.analytics_logger.log_retrieval(
            &query,
            collection_id,
            results.len(),
            average_score(&results),
            start.elapsed().as_millis() as u64,
        );

        return Ok(HybridRetrievalResponse { results, cache_hit });
    }

    let results = if optimized {
        state
            .retrieval_service
            .query_optimized_with_config(collection_id, &query, top_k, &config, &state.logs)
            .await
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "RAG",
                    &format!("Hybrid retrieval (optimized) failed: {}", e),
                );
                e
            })?
    } else {
        state
            .retrieval_service
            .query(collection_id, &query, top_k)
            .await
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "RAG",
                    &format!("Hybrid retrieval failed: {}", e),
                );
                e
            })?
    };

    state.analytics_logger.log_retrieval(
        &query,
        collection_id,
        results.len(),
        average_score(&results),
        start.elapsed().as_millis() as u64,
    );

    Ok(HybridRetrievalResponse {
        results,
        cache_hit: false,
    })
}

// ============================================================
// PHASE 7: TESTING & VALIDATION
// ============================================================


#[tauri::command]
pub async fn rag_run_validation_suite(
    state: State<'_, Arc<super::AppState>>,
    request: ValidationSuiteRequest,
) -> Result<ValidationReport> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Running validation suite ({} cases)", request.cases.len()),
    );

    let options = request.options.unwrap_or_default();
    RagValidationSuite::run(
        &state.retrieval_service,
        &state.rag_ingestion_use_case,
        &request.cases,
        &options,
    )
    .await
    .map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Validation suite failed: {}", e),
        );
        e
    })
}

// ============================================================
// PHASE 8: LOGGING & ANALYTICS
// ============================================================


