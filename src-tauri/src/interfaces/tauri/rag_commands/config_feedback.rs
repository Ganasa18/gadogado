//! Configuration and Feedback Commands
//!
//! This module provides Tauri commands for:
//! - RAG configuration management (get, update, reset, validate)
//! - User feedback collection and statistics

use crate::application::use_cases::rag_config::{
    CacheConfig, ChatConfig, ChunkingConfig, ConfigValidation, EmbeddingConfig, FeedbackRating,
    FeedbackStats, OcrConfig, RagConfig, RetrievalConfig, UserFeedback,
};
use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

use super::analytics_cache_metrics::truncate_message;

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

