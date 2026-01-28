//! RAG Settings Commands
//!
//! Tauri commands for managing RAG context settings and model context limits.

use crate::domain::context_config::{ContextWindowConfig, ModelContextLimit};
use crate::domain::error::Result;
use tauri::State;
use std::sync::Arc;

/// Get global RAG context settings
#[tauri::command]
pub async fn get_rag_global_settings(
    state: State<'_, Arc<super::AppState>>,
) -> std::result::Result<ContextWindowConfig, String> {
    state
        .rag_repository
        .get_global_settings()
        .await
        .map_err(|e| e.to_string())
}

/// Update global RAG context settings
#[tauri::command]
pub async fn update_rag_global_settings(
    settings: ContextWindowConfig,
    state: State<'_, Arc<super::AppState>>,
) -> std::result::Result<(), String> {
    state
        .rag_repository
        .update_global_settings(&settings)
        .await
        .map_err(|e| e.to_string())
}

/// Get model context limit for specific provider/model
#[tauri::command]
pub async fn get_model_context_limit(
    provider: String,
    model_name: String,
    state: State<'_, Arc<super::AppState>>,
) -> std::result::Result<ModelContextLimit, String> {
    state
        .rag_repository
        .get_or_infer_limit(&provider, &model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Get all available model limits from database
#[tauri::command]
pub async fn get_all_model_limits(
    state: State<'_, Arc<super::AppState>>,
) -> std::result::Result<Vec<ModelContextLimit>, String> {
    state
        .rag_repository
        .get_all_model_limits()
        .await
        .map_err(|e| e.to_string())
}

/// Insert or update a model context limit
#[tauri::command]
pub async fn upsert_model_limit(
    provider: String,
    model_name: String,
    context_window: usize,
    max_output_tokens: usize,
    state: State<'_, Arc<super::AppState>>,
) -> std::result::Result<i64, String> {
    state
        .rag_repository
        .upsert_model_limit(&provider, &model_name, context_window, max_output_tokens)
        .await
        .map_err(|e| e.to_string())
}
