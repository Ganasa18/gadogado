//! Search and Chat Commands
//!
//! This module provides Tauri commands for:
//! - Hybrid search with vector and keyword retrieval
//! - RAG query with prompt building
//! - Conversational chat with context
//! - Answer verification and correction

use crate::application::use_cases::prompt_engine::{PromptEngine, VerificationResult};
use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

use super::analytics_cache_metrics::truncate_message;
use super::chunks::average_score;
use super::types::*;

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

    let mut config = state.config_manager.get_config();
    if let Some(k) = request.candidate_k {
        config.retrieval.candidate_k = k;
    }
    if let Some(k) = request.rerank_k {
        config.retrieval.rerank_k = k;
    }

    let top_k = request.top_k.unwrap_or(config.retrieval.top_k);
    let results = state
        .retrieval_service
        .query_optimized_with_config(
            request.collection_id,
            &request.query,
            top_k,
            &config,
            &state.logs,
        )
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
    let config = state.config_manager.get_config();
    let results = state
        .retrieval_service
        .query_optimized_with_config(
            request.collection_id,
            &request.query,
            top_k,
            &config,
            &state.logs,
        )
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

