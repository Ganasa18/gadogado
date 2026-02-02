//! Search and Chat Commands
//!
//! This module provides Tauri commands for:
//! - Hybrid search with vector and keyword retrieval
//! - RAG query with prompt building
//! - Conversational chat with context
//! - Answer verification and correction

use crate::application::use_cases::prompt_engine::{PromptEngine, VerificationResult};
use crate::application::use_cases::context_manager::{ContextManager, BuildContext};
use crate::application::use_cases::conversation_service::ConversationMessage;
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

    let language = request.language.as_deref();
    let prompt = if request.enable_few_shot.unwrap_or(false) {
        PromptEngine::build_conversational_nl_prompt_with_few_shot(
            &request.query,
            &results,
            language,
        )
    } else {
        PromptEngine::build_conversational_nl_prompt(&request.query, &results, language)
    }
    .map_err(|e| {
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

    // Extract language for conversational responses
    let language = request.language.as_deref();

    // Build RAG context string from results
    let rag_context = results
        .iter()
        .map(|r| {
            let score_str = r.score.map_or("N/A".to_string(), |s| format!("{:.2}", s));
            format!("Content: {}\nScore: {}", r.content, score_str)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Build prompt with context management
    let (prompt, context_summary, context_managed) = if let Some(ref messages) = request.messages {
        // Load global RAG settings and model limit for context management
        let global_settings = state.rag_repository.get_global_settings().await
            .unwrap_or_else(|e| {
                add_log(
                    &state.logs,
                    "WARN",
                    "RAG",
                    &format!("Failed to load global settings, using defaults: {}", e),
                );
                crate::domain::context_config::ContextWindowConfig::default()
            });

        // Get provider/model from request or use defaults
        let provider = request.provider.as_deref().unwrap_or("local").to_string();
        let model = request.model.as_deref().unwrap_or("default").to_string();

        // Get model context limit
        let model_limit = state.rag_repository.get_or_infer_limit(&provider, &model).await
            .unwrap_or_else(|e| {
                add_log(
                    &state.logs,
                    "WARN",
                    "RAG",
                    &format!("Failed to get model limit, using defaults: {}", e),
                );
                crate::domain::context_config::ModelContextLimit {
                    id: 0,
                    provider: provider.clone(),
                    model_name: model.clone(),
                    context_window: 8000,
                    max_output_tokens: 2048,
                }
            });

        // Create context manager
        let context_manager = ContextManager::new(
            global_settings,
            provider.clone(),
            model.clone(),
            model_limit.context_window,
        );

        // Convert ChatMessage to ConversationMessage format
        let conversation_messages: Vec<ConversationMessage> = messages
            .iter()
            .enumerate()
            .map(|(i, m)| ConversationMessage {
                id: i as i64,
                conversation_id: request.conversation_id.unwrap_or(0),
                role: m.role.clone(),
                content: m.content.clone(),
                sources: None,
                created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            })
            .collect();

        // Use ContextManager to build context
        let build_context = context_manager.build_context(conversation_messages, &rag_context)
            .unwrap_or_else(|e| {
                add_log(
                    &state.logs,
                    "WARN",
                    "RAG",
                    &format!("Context management failed, using simple approach: {}", e),
                );
                // Fallback to simple context
                BuildContext {
                    messages: messages.iter().skip(messages.len().saturating_sub(3)).cloned()
                        .map(|m| ConversationMessage {
                            id: 0,
                            conversation_id: 0,
                            role: m.role.clone(),
                            content: m.content.clone(),
                            sources: None,
                            created_at: String::new(),
                        }).collect(),
                    summary: None,
                    token_estimate: rag_context.len() / 4,
                    was_compacted: false,
                    strategy_used: crate::domain::context_config::CompactionStrategy::Truncate,
                }
            });

        // Convert managed messages back to tuples
        let message_tuples: Vec<(String, String)> = build_context.messages
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();

        // Use new conversational prompt with managed chat history
        let prompt = PromptEngine::build_conversational_prompt_with_chat(
            &request.query,
            &results,
            build_context.summary.as_deref(),
            &message_tuples,
            language,
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

        add_log(
            &state.logs,
            "INFO",
            "RAG",
            &format!(
                "Context managed: compacted={}, strategy={}, tokens={}, messages={}/{}",
                build_context.was_compacted,
                build_context.strategy_used,
                build_context.token_estimate,
                build_context.messages.len(),
                request.messages.as_ref().map(|m| m.len()).unwrap_or(0)
            ),
        );

        let context_info = ContextManagedInfo {
            was_compacted: build_context.was_compacted,
            strategy_used: build_context.strategy_used.to_string(),
            token_estimate: build_context.token_estimate,
            messages_used: build_context.messages.len(),
            messages_total: request.messages.as_ref().map(|m| m.len()).unwrap_or(0),
        };

        (prompt, build_context.summary, Some(context_info))
    } else {
        // Use conversational NL prompt (no chat history)
        let prompt =
            PromptEngine::build_conversational_nl_prompt(&request.query, &results, language)
                .map_err(|e| {
                    add_log(
                        &state.logs,
                        "ERROR",
                        "RAG",
                        &format!("Prompt building failed: {}", e),
                    );
                    e
                })?;

        (prompt, None, None)
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
        context_managed,
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

