use crate::application::use_cases::chunking::{ChunkConfig, ChunkEngine, ChunkStrategy};
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
use crate::domain::error::Result;
use crate::domain::rag_entities::{
    RagCollection, RagCollectionInput, RagDocument, RagDocumentChunk, RagExcelData,
};
use crate::interfaces::http::add_log;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct RagQueryRequest {
    pub collection_id: i64,
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct RagQueryResponse {
    pub prompt: String,
    pub results: Vec<crate::application::QueryResult>,
}

// ============================================================
// PHASE 6: BACKEND API EXTENSIONS
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedOcrRequest {
    pub file_path: String,
    pub config: Option<OcrConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SmartChunkingRequest {
    pub text: String,
    pub config: Option<ChunkingConfig>,
}

#[derive(Debug, Serialize)]
pub struct SmartChunk {
    pub index: usize,
    pub content: String,
    pub token_count: usize,
    pub quality_score: Option<f32>,
    pub content_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HybridRetrievalOptions {
    pub top_k: Option<usize>,
    pub use_cache: Option<bool>,
    pub optimized: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct HybridRetrievalResponse {
    pub results: Vec<crate::application::QueryResult>,
    pub cache_hit: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationSuiteRequest {
    pub cases: Vec<ValidationCase>,
    pub options: Option<ValidationOptions>,
}

/// Web crawl mode: "html" (fast) or "ocr" (accurate for JS-heavy sites)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebCrawlMode {
    #[default]
    Html,
    Ocr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagWebImportRequest {
    pub url: String,
    pub collection_id: Option<i64>,
    pub max_pages: Option<usize>,
    pub max_depth: Option<usize>,
    /// Crawl mode: "html" (default) or "ocr" (Playwright + Tesseract)
    #[serde(default)]
    pub mode: WebCrawlMode,
}

#[tauri::command]
pub async fn rag_create_collection(
    state: State<'_, Arc<super::AppState>>,
    input: RagCollectionInput,
) -> Result<RagCollection> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Creating collection: {}", input.name),
    );

    state
        .rag_repository
        .create_collection(&input)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to create collection: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_get_collection(
    state: State<'_, Arc<super::AppState>>,
    id: i64,
) -> Result<RagCollection> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Fetching collection: {}", id),
    );

    state.rag_repository.get_collection(id).await.map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Failed to fetch collection: {}", e),
        );
        e
    })
}

#[tauri::command]
pub async fn rag_list_collections(
    state: State<'_, Arc<super::AppState>>,
    limit: Option<i64>,
) -> Result<Vec<RagCollection>> {
    add_log(&state.logs, "INFO", "RAG", "Listing collections");

    state
        .rag_repository
        .list_collections(limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list collections: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_delete_collection(state: State<'_, Arc<super::AppState>>, id: i64) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting collection: {}", id),
    );

    state
        .rag_repository
        .delete_collection(id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to delete collection: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_get_document(
    state: State<'_, Arc<super::AppState>>,
    id: i64,
) -> Result<RagDocument> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Fetching document: {}", id),
    );

    state.rag_repository.get_document(id).await.map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Failed to fetch document: {}", e),
        );
        e
    })
}

#[tauri::command]
pub async fn rag_delete_document(state: State<'_, Arc<super::AppState>>, id: i64) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting document: {}", id),
    );

    let rows = state
        .rag_repository
        .delete_document(id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to delete document: {}", e),
            );
            e
        })?;

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleted document: {} (rows {})", id, rows),
    );

    Ok(rows)
}

#[tauri::command]
pub async fn rag_list_documents(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<RagDocument>> {
    add_log(&state.logs, "INFO", "RAG", "Listing documents");

    state
        .rag_repository
        .list_documents(collection_id, limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list documents: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_import_file(
    state: State<'_, Arc<super::AppState>>,
    file_path: String,
    collection_id: Option<i64>,
) -> Result<RagDocument> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Importing file: {}", file_path),
    );

    let start = Instant::now();
    let result = state
        .rag_ingestion_use_case
        .ingest_file(&file_path, collection_id, state.logs.clone())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to import file: {}", e),
            );
            e
        });

    let doc_type = Path::new(&file_path)
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
pub async fn rag_list_chunks(
    state: State<'_, Arc<super::AppState>>,
    doc_id: i64,
    limit: Option<i64>,
) -> Result<Vec<RagDocumentChunk>> {
    add_log(&state.logs, "INFO", "RAG", "Listing chunks");

    state
        .rag_repository
        .get_chunks(doc_id, limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list chunks: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_list_excel_data(
    state: State<'_, Arc<super::AppState>>,
    doc_id: i64,
    limit: Option<i64>,
) -> Result<Vec<RagExcelData>> {
    add_log(&state.logs, "INFO", "RAG", "Listing Excel data");

    state
        .rag_repository
        .get_excel_data(doc_id, limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list Excel data: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_hybrid_search(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<crate::application::QueryResult>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Hybrid search in collection {}: {}", collection_id, query),
    );

    let start = Instant::now();
    let results = state
        .retrieval_service
        .query(collection_id, &query, top_k.unwrap_or(5))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Hybrid search failed: {}", e),
            );
            e
        })?;

    state.analytics_logger.log_retrieval(
        &query,
        collection_id,
        results.len(),
        average_score(&results),
        start.elapsed().as_millis() as u64,
    );

    Ok(results)
}

#[tauri::command]
pub async fn rag_query(
    state: State<'_, Arc<super::AppState>>,
    request: RagQueryRequest,
) -> Result<RagQueryResponse> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Query in collection {}: {}",
            request.collection_id, request.query
        ),
    );

    let start = Instant::now();
    let doc_count = state
        .rag_repository
        .list_documents(Some(request.collection_id), 1000)
        .await
        .map(|docs| docs.len())
        .unwrap_or(0);
    let chunks = state
        .rag_repository
        .search_chunks_by_collection(request.collection_id, 1000)
        .await;
    match chunks {
        Ok(chunks) => {
            let embedded_count = chunks
                .iter()
                .filter(|chunk| chunk.embedding.is_some())
                .count();
            add_log(
                &state.logs,
                "INFO",
                "RAG",
                &format!(
                    "Collection {} has {} documents, {} chunks ({} with embeddings)",
                    request.collection_id,
                    doc_count,
                    chunks.len(),
                    embedded_count
                ),
            );
        }
        Err(err) => {
            add_log(
                &state.logs,
                "WARN",
                "RAG",
                &format!(
                    "Failed to inspect collection {}: {}",
                    request.collection_id, err
                ),
            );
        }
    }

    let top_k = request.top_k.unwrap_or(5);
    let results = state
        .retrieval_service
        .query(request.collection_id, &request.query, top_k)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Retrieval failed: {}", e),
            );
            e
        })?;

    state.analytics_logger.log_retrieval(
        &request.query,
        request.collection_id,
        results.len(),
        average_score(&results),
        start.elapsed().as_millis() as u64,
    );

    let prompt = PromptEngine::build_prompt(&request.query, &results).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Prompt building failed: {}", e),
        );
        e
    })?;

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Built prompt with {} results", results.len()),
    );

    Ok(RagQueryResponse { prompt, results })
}

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
            .query_optimized(collection_id, &query, top_k)
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

// ============================================================
// CHAT WITH CONTEXT (Conversational RAG)
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatWithContextRequest {
    pub collection_id: i64,
    pub query: String,
    pub conversation_id: Option<i64>,
    pub messages: Option<Vec<ChatMessage>>,
    pub top_k: Option<usize>,
    /// Enable self-correction mode
    #[serde(default)]
    pub enable_verification: bool,
}

#[derive(Debug, Serialize)]
pub struct ChatWithContextResponse {
    pub prompt: String,
    pub results: Vec<crate::application::QueryResult>,
    pub conversation_id: Option<i64>,
    /// Conversation context summary (if applicable)
    pub context_summary: Option<String>,
    /// Whether the answer was verified
    pub verified: bool,
}

#[tauri::command]
pub async fn rag_chat_with_context(
    state: State<'_, Arc<super::AppState>>,
    request: ChatWithContextRequest,
) -> Result<ChatWithContextResponse> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Chat with context in collection {}: {}",
            request.collection_id, request.query
        ),
    );

    let start = Instant::now();
    let top_k = request.top_k.unwrap_or(5);

    // Use optimized query for better results
    let results = state
        .retrieval_service
        .query_optimized(request.collection_id, &request.query, top_k)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Retrieval failed: {}", e),
            );
            e
        })?;

    // Build prompt based on whether we have conversation history
    let (prompt, context_summary) = if let Some(ref messages) = request.messages {
        // Convert messages to tuple format for conversational prompt
        let message_tuples: Vec<(String, String)> = messages
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();

        // Build a simple summary from previous messages
        let summary = if messages.len() > 3 {
            Some(
                messages
                    .iter()
                    .take(messages.len().saturating_sub(3))
                    .map(|m| format!("{}: {}", m.role, truncate_message(&m.content, 100)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        } else {
            None
        };

        let prompt = PromptEngine::build_conversational_prompt(
            &request.query,
            &results,
            summary.as_deref(),
            &message_tuples[message_tuples.len().saturating_sub(3)..],
        )
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Prompt building failed: {}", e),
            );
            e
        })?;

        (prompt, summary)
    } else {
        // Use reflective prompt for better quality
        let prompt =
            PromptEngine::build_reflective_prompt(&request.query, &results).map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "RAG",
                    &format!("Prompt building failed: {}", e),
                );
                e
            })?;

        (prompt, None)
    };

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Built chat prompt with {} results, verification: {}",
            results.len(),
            request.enable_verification
        ),
    );

    let duration_ms = start.elapsed().as_millis() as u64;
    state.analytics_logger.log_retrieval(
        &request.query,
        request.collection_id,
        results.len(),
        average_score(&results),
        duration_ms,
    );
    state.analytics_logger.log_chat(
        &request.query,
        Some(request.collection_id),
        0,
        None,
        duration_ms,
    );

    Ok(ChatWithContextResponse {
        prompt,
        results,
        conversation_id: request.conversation_id,
        context_summary,
        verified: false, // Verification happens client-side with LLM
    })
}

/// Build verification prompt for self-correcting RAG
#[tauri::command]
pub async fn rag_build_verification_prompt(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    query: String,
    answer: String,
    top_k: Option<usize>,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        "Building verification prompt for answer",
    );

    let results = state
        .retrieval_service
        .query(collection_id, &query, top_k.unwrap_or(5))
        .await?;

    let prompt = PromptEngine::build_verification_prompt(&query, &answer, &results);

    Ok(prompt)
}

/// Build correction prompt based on verification result
#[tauri::command]
pub async fn rag_build_correction_prompt(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    query: String,
    original_answer: String,
    verification: VerificationResult,
    top_k: Option<usize>,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Building correction prompt: {}", verification.summary()),
    );

    let results = state
        .retrieval_service
        .query(collection_id, &query, top_k.unwrap_or(5))
        .await?;

    let prompt =
        PromptEngine::build_correction_prompt(&query, &original_answer, &verification, &results);

    Ok(prompt)
}

// ============================================================
// HEALTH CHECK
// ============================================================

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub status: String,
    pub components: HealthComponents,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct HealthComponents {
    pub database: ComponentHealth,
    pub embedding_service: ComponentHealth,
    pub embedding_cache: CacheHealth,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CacheHealth {
    pub status: String,
    pub total_entries: usize,
    pub valid_entries: usize,
    pub max_size: usize,
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
fn truncate_message(text: &str, max_len: usize) -> String {
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

/// Get current RAG configuration
#[tauri::command]
pub async fn rag_get_config(state: State<'_, Arc<super::AppState>>) -> Result<RagConfig> {
    add_log(&state.logs, "INFO", "RAG", "Getting RAG configuration");
    let config = state.config_manager.get_config();
    Ok(config)
}

/// Update entire RAG configuration
#[tauri::command]
pub async fn rag_update_config(
    state: State<'_, Arc<super::AppState>>,
    config: RagConfig,
) -> Result<ConfigValidation> {
    add_log(&state.logs, "INFO", "RAG", "Updating RAG configuration");

    let validation = state.config_manager.update_config(config);

    if validation.valid {
        // Save to file
        if let Err(e) = state.config_manager.save() {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to save config: {}", e),
            );
        }
        add_log(
            &state.logs,
            "INFO",
            "RAG",
            "Configuration updated successfully",
        );
    } else {
        add_log(
            &state.logs,
            "WARN",
            "RAG",
            &format!("Configuration validation failed: {:?}", validation.errors),
        );
    }

    Ok(validation)
}

/// Update chunking configuration
#[tauri::command]
pub async fn rag_update_chunking_config(
    state: State<'_, Arc<super::AppState>>,
    config: ChunkingConfig,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        "Updating chunking configuration",
    );
    state.config_manager.update_chunking(config);
    let _ = state.config_manager.save();
    Ok("Chunking configuration updated".to_string())
}

/// Update retrieval configuration
#[tauri::command]
pub async fn rag_update_retrieval_config(
    state: State<'_, Arc<super::AppState>>,
    config: RetrievalConfig,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        "Updating retrieval configuration",
    );
    state.config_manager.update_retrieval(config);
    let _ = state.config_manager.save();
    Ok("Retrieval configuration updated".to_string())
}

/// Update embedding configuration
#[tauri::command]
pub async fn rag_update_embedding_config(
    state: State<'_, Arc<super::AppState>>,
    config: EmbeddingConfig,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        "Updating embedding configuration",
    );
    state.config_manager.update_embedding(config);
    let _ = state.config_manager.save();
    Ok("Embedding configuration updated".to_string())
}

/// Update OCR configuration
#[tauri::command]
pub async fn rag_update_ocr_config(
    state: State<'_, Arc<super::AppState>>,
    config: OcrConfig,
) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Updating OCR configuration");
    state.config_manager.update_ocr(config);
    let _ = state.config_manager.save();
    Ok("OCR configuration updated".to_string())
}

/// Update cache configuration
#[tauri::command]
pub async fn rag_update_cache_config(
    state: State<'_, Arc<super::AppState>>,
    config: CacheConfig,
) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Updating cache configuration");
    state.config_manager.update_cache(config);
    let _ = state.config_manager.save();
    Ok("Cache configuration updated".to_string())
}

/// Update chat configuration
#[tauri::command]
pub async fn rag_update_chat_config(
    state: State<'_, Arc<super::AppState>>,
    config: ChatConfig,
) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Updating chat configuration");
    state.config_manager.update_chat(config);
    let _ = state.config_manager.save();
    Ok("Chat configuration updated".to_string())
}

/// Reset RAG configuration to defaults
#[tauri::command]
pub async fn rag_reset_config(state: State<'_, Arc<super::AppState>>) -> Result<RagConfig> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        "Resetting RAG configuration to defaults",
    );
    state.config_manager.reset_to_defaults();
    let _ = state.config_manager.save();
    let config = state.config_manager.get_config();
    Ok(config)
}

/// Validate current configuration
#[tauri::command]
pub async fn rag_validate_config(
    state: State<'_, Arc<super::AppState>>,
) -> Result<ConfigValidation> {
    let validation = state.config_manager.validate();
    Ok(validation)
}

// ============================================================
// USER FEEDBACK
// ============================================================

/// Submit user feedback for a RAG response
#[tauri::command]
pub async fn rag_submit_feedback(
    state: State<'_, Arc<super::AppState>>,
    feedback: UserFeedback,
) -> Result<String> {
    let rating_str = match feedback.rating {
        FeedbackRating::ThumbsUp => "positive",
        FeedbackRating::ThumbsDown => "negative",
        FeedbackRating::Neutral => "neutral",
    };

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Received {} feedback for query: {}",
            rating_str,
            truncate_message(&feedback.query_text, 50)
        ),
    );

    let query_text = feedback.query_text.clone();
    let response_len = feedback.response_text.len();
    let collection_id = feedback.collection_id;
    state.feedback_collector.add_feedback(feedback);
    state.analytics_logger.log_chat(
        &query_text,
        collection_id,
        response_len,
        Some(rating_str.to_string()),
        0,
    );
    Ok("Feedback submitted successfully".to_string())
}

/// Get feedback statistics
#[tauri::command]
pub async fn rag_get_feedback_stats(
    state: State<'_, Arc<super::AppState>>,
) -> Result<FeedbackStats> {
    let stats = state.feedback_collector.get_stats();
    Ok(stats)
}

/// Get recent feedback entries
#[tauri::command]
pub async fn rag_get_recent_feedback(
    state: State<'_, Arc<super::AppState>>,
    limit: Option<usize>,
) -> Result<Vec<UserFeedback>> {
    let feedback = state
        .feedback_collector
        .get_recent_feedback(limit.unwrap_or(20));
    Ok(feedback)
}

/// Clear all feedback
#[tauri::command]
pub async fn rag_clear_feedback(state: State<'_, Arc<super::AppState>>) -> Result<String> {
    add_log(&state.logs, "INFO", "RAG", "Clearing user feedback");
    state.feedback_collector.clear();
    Ok("Feedback cleared successfully".to_string())
}

// ============================================================
// CHUNK MANAGEMENT (for ChunkViewer UI)
// ============================================================

#[derive(Debug, Serialize)]
pub struct ChunkWithQuality {
    pub chunk: RagDocumentChunk,
    pub quality_score: f32,
    pub has_embedding: bool,
    pub token_estimate: usize,
}

/// Get chunks for a document with quality information
#[tauri::command]
pub async fn rag_get_chunks_with_quality(
    state: State<'_, Arc<super::AppState>>,
    document_id: i64,
    limit: Option<i64>,
) -> Result<Vec<ChunkWithQuality>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting chunks with quality for document: {}", document_id),
    );

    // Get chunks with embeddings to check which have embeddings
    let chunks_with_embeddings = state
        .rag_repository
        .get_chunks_with_embeddings(document_id)
        .await?;

    // Create a set of chunk IDs that have embeddings
    let chunks_with_embedding_ids: std::collections::HashSet<i64> = chunks_with_embeddings
        .iter()
        .filter(|(_, _, _, _, emb)| emb.is_some())
        .map(|(id, _, _, _, _)| *id)
        .collect();

    let chunks = state
        .rag_repository
        .get_chunks(document_id, limit.unwrap_or(1000))
        .await?;

    let chunks_with_quality: Vec<ChunkWithQuality> = chunks
        .into_iter()
        .map(|chunk| {
            let quality_score = estimate_chunk_quality(&chunk.content);
            let has_embedding = chunks_with_embedding_ids.contains(&chunk.id);
            let token_estimate = chunk.content.len() / 4; // ~4 chars per token

            ChunkWithQuality {
                chunk,
                quality_score,
                has_embedding,
                token_estimate,
            }
        })
        .collect();

    Ok(chunks_with_quality)
}

/// Delete a specific chunk
#[tauri::command]
pub async fn rag_delete_chunk(
    state: State<'_, Arc<super::AppState>>,
    chunk_id: i64,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting chunk: {}", chunk_id),
    );

    state.rag_repository.delete_chunk(chunk_id).await
}

/// Update chunk content (for manual editing)
#[tauri::command]
pub async fn rag_update_chunk_content(
    state: State<'_, Arc<super::AppState>>,
    chunk_id: i64,
    new_content: String,
) -> Result<RagDocumentChunk> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Updating chunk content: {}", chunk_id),
    );

    // Update content and regenerate embedding
    state
        .rag_repository
        .update_chunk_content(chunk_id, &new_content)
        .await?;

    // Regenerate embedding for the updated content
    if let Ok(embedding) = state
        .embedding_service
        .generate_embedding(&new_content)
        .await
    {
        let embedding_bytes =
            crate::application::use_cases::embedding_service::EmbeddingService::embedding_to_bytes(
                &embedding,
            );
        let _ = state
            .rag_repository
            .update_chunk_embedding(chunk_id, &embedding_bytes)
            .await;
    }

    // Return updated chunk
    state.rag_repository.get_chunk(chunk_id).await
}

/// Re-embed a chunk (regenerate embedding)
#[tauri::command]
pub async fn rag_reembed_chunk(
    state: State<'_, Arc<super::AppState>>,
    chunk_id: i64,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Re-embedding chunk: {}", chunk_id),
    );

    let chunk = state.rag_repository.get_chunk(chunk_id).await?;

    let embedding = state
        .embedding_service
        .generate_embedding(&chunk.content)
        .await
        .map_err(|e| {
            crate::domain::error::AppError::Internal(format!("Embedding failed: {}", e))
        })?;

    let embedding_bytes =
        crate::application::use_cases::embedding_service::EmbeddingService::embedding_to_bytes(
            &embedding,
        );
    state
        .rag_repository
        .update_chunk_embedding(chunk_id, &embedding_bytes)
        .await?;

    Ok("Chunk re-embedded successfully".to_string())
}

/// Filter chunks by quality threshold
#[tauri::command]
pub async fn rag_filter_low_quality_chunks(
    state: State<'_, Arc<super::AppState>>,
    document_id: i64,
    quality_threshold: f32,
) -> Result<Vec<ChunkWithQuality>> {
    let all_chunks = rag_get_chunks_with_quality(state.clone(), document_id, Some(10000)).await?;

    let low_quality: Vec<ChunkWithQuality> = all_chunks
        .into_iter()
        .filter(|c| c.quality_score < quality_threshold)
        .collect();

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Found {} low quality chunks (threshold: {})",
            low_quality.len(),
            quality_threshold
        ),
    );

    Ok(low_quality)
}

// Helper function to estimate chunk quality (replicated from rag_ingestion)
fn estimate_chunk_quality(content: &str) -> f32 {
    let mut score = 0.0f32;
    let len = content.len();

    // Length score (prefer 100-500 chars)
    if len >= 100 && len <= 500 {
        score += 0.3;
    } else if len >= 50 && len <= 800 {
        score += 0.2;
    } else if len < 50 {
        score += 0.05;
    } else {
        score += 0.1;
    }

    // Content quality indicators
    let has_alphanumeric = content.chars().any(|c| c.is_alphanumeric());
    let alpha_ratio =
        content.chars().filter(|c| c.is_alphabetic()).count() as f32 / len.max(1) as f32;
    let has_sentences = content.contains('.') || content.contains('!') || content.contains('?');
    let has_capital = content.chars().any(|c| c.is_uppercase());

    if has_alphanumeric {
        score += 0.2;
    }
    if alpha_ratio > 0.5 {
        score += 0.2;
    }
    if has_sentences {
        score += 0.15;
    }
    if has_capital {
        score += 0.15;
    }

    score.min(1.0)
}

fn average_score(results: &[crate::application::QueryResult]) -> Option<f32> {
    let mut total = 0.0f32;
    let mut count = 0usize;

    for result in results {
        if let Some(score) = result.score {
            total += score;
            count += 1;
        }
    }

    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}

// ============================================================
// CONVERSATION PERSISTENCE
// ============================================================

use crate::application::use_cases::conversation_service::{Conversation, ConversationMessage};

/// Create a new conversation
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

/// Get collection quality metrics
#[tauri::command]
pub async fn rag_get_collection_quality(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<Option<CollectionQualityMetrics>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting quality metrics for collection {}", collection_id),
    );

    state
        .rag_repository
        .get_collection_quality_metrics(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get collection quality: {}", e),
            );
            e
        })
}

/// Compute and refresh collection quality metrics
#[tauri::command]
pub async fn rag_compute_collection_quality(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<CollectionQualityMetrics> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Computing quality metrics for collection {}", collection_id),
    );

    state
        .rag_repository
        .compute_collection_quality_metrics(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to compute collection quality: {}", e),
            );
            e
        })
}

/// Get document warnings
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
