use crate::application::use_cases::audit_service::AuditService;
use crate::application::use_cases::data_protection::DataProtectionService;
use crate::application::use_cases::db_connection_manager::DbConnectionManager;
use crate::application::use_cases::enhance::EnhanceUseCase;
use crate::application::use_cases::qa_ai::{ExploreResult, QaAiUseCase};
use crate::application::use_cases::qa_api_call::QaApiCallUseCase;
use crate::application::use_cases::qa_event::QaEventUseCase;
use crate::application::use_cases::qa_run::QaRunUseCase;
use crate::application::use_cases::qa_session::QaSessionUseCase;
use crate::application::use_cases::rag_ingestion::RagIngestionUseCase;
use crate::application::use_cases::rate_limiter::RateLimiter;
use crate::application::use_cases::retrieval_service::RetrievalService;
use crate::application::use_cases::translate::TranslateUseCase;
use crate::application::use_cases::typegen::TypeGenUseCase;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::{ChatMessage, LLMConfig};
use crate::domain::prompt::Prompt;
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaLlmRun, QaTestCase};
use crate::domain::qa_event::{QaEvent, QaEventInput, QaEventPage};
use crate::domain::qa_run::{QaRunStreamEvent, QaRunStreamInput, QaSessionRun};
use crate::domain::qa_session::QaSession;
use crate::infrastructure::db::rag::repository::RagRepository;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::storage::{
    ensure_qa_sessions_root, ensure_session_dir, ensure_session_screenshots_dir,
    resolve_app_data_dir,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tracing::error;
use uuid::Uuid;

use crate::application::use_cases::conversation_service::ConversationService;
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::rag_analytics::SharedAnalyticsLogger;
use crate::application::use_cases::rag_config::{SharedConfigManager, SharedFeedbackCollector};
use crate::application::use_cases::rag_metrics::{SharedExperimentManager, SharedMetricsCollector};
use crate::infrastructure::config::ConfigService;
use crate::infrastructure::llm_clients::LLMClient;
use crate::interfaces::http::{add_log, add_log_entry, LogEntry};
use crate::interfaces::mock_server::{
    build_status as build_mock_status, save_config as save_mock_server_config, start_mock_server,
    stop_mock_server, MockServerConfig, MockServerState, MockServerStatus,
};

use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JsonValue};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::Mutex as AsyncMutex;

use super::state::AppState;


#[tauri::command]
pub async fn translate_prompt(
    state: State<'_, Arc<AppState>>,
    config: LLMConfig,
    content: String,
    source: String,
    target: String,
) -> Result<Prompt> {
    state
        .translate_use_case
        .execute(&config, content, source, target)
        .await
}

#[tauri::command]
pub async fn enhance_prompt(
    state: State<'_, Arc<AppState>>,
    config: LLMConfig,
    content: String,
    system_prompt: Option<String>,
) -> Result<Prompt> {
    state
        .enhance_use_case
        .execute(&config, content, system_prompt)
        .await
}

#[tauri::command]
pub async fn llm_chat(
    state: State<'_, Arc<AppState>>,
    config: LLMConfig,
    messages: Vec<ChatMessage>,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "LLM",
        &format!(
            "LLM chat request - provider: {}, model: {}, messages: {}",
            config.provider,
            config.model,
            messages.len()
        ),
    );

    // Extract system and user messages
    let mut system_message = String::from("You are a helpful AI assistant.");
    let mut user_messages = Vec::new();

    for msg in &messages {
        match msg.role.as_str() {
            "system" => system_message = msg.content.clone(),
            "user" => user_messages.push(msg.content.clone()),
            _ => {}
        }
    }

    // Join user messages with newlines
    let user_message = user_messages.join("\n");

    // Use the translate use case as a wrapper for general LLM calls
    // In production, this should be its own dedicated use case
    state
        .translate_use_case
        .execute(&config, user_message, String::new(), system_message)
        .await
        .map(|p| p.content)
}

#[tauri::command]
pub async fn get_translation_history(
    state: State<'_, Arc<AppState>>,
    limit: i64,
) -> Result<Vec<Prompt>> {
    state.repository.get_history(limit).await
}

#[tauri::command]
pub async fn save_api_key(
    state: State<'_, Arc<AppState>>,
    provider: String,
    key: String,
) -> Result<()> {
    state.config_service.save_api_key(&provider, &key)
}

#[tauri::command]
pub async fn get_api_key(state: State<'_, Arc<AppState>>, provider: String) -> Result<String> {
    state.config_service.get_api_key(&provider)
}

#[tauri::command]
pub async fn delete_api_key(state: State<'_, Arc<AppState>>, provider: String) -> Result<()> {
    state.config_service.delete_api_key(&provider)
}

#[tauri::command]
pub async fn add_log_message(
    state: State<'_, Arc<AppState>>,
    level: String,
    source: String,
    message: String,
) -> Result<()> {
    add_log(&state.logs, &level, &source, &message);
    Ok(())
}

#[tauri::command]
pub async fn get_llm_models(
    state: State<'_, Arc<AppState>>,
    config: LLMConfig,
) -> Result<Vec<String>> {
    state.llm_client.list_models(&config).await
}

#[tauri::command]
pub async fn sync_config(state: State<'_, Arc<AppState>>, config: LLMConfig) -> Result<()> {
    let mut last_config = state.last_config.lock().unwrap();
    *last_config = config.clone();
    add_log(
        &state.logs,
        "INFO",
        "Config",
        &format!(
            "Synced config: provider={:?} base_url={} model={}",
            last_config.provider, last_config.base_url, last_config.model
        ),
    );
    Ok(())
}

#[tauri::command]
pub async fn sync_embedding_config(
    state: State<'_, Arc<AppState>>,
    config: LLMConfig,
) -> Result<()> {
    state.embedding_service.update_config(config.clone());
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Embedding service configured with provider={:?}, model={}",
            config.provider, config.model
        ),
    );
    Ok(())
}

#[tauri::command]
pub async fn sync_languages(
    state: State<'_, Arc<AppState>>,
    source: String,
    target: String,
) -> Result<()> {
    *state.preferred_source.lock().unwrap() = source;
    *state.preferred_target.lock().unwrap() = target;
    Ok(())
}

#[tauri::command]
pub fn sync_shortcuts(
    app: tauri::AppHandle,
    enabled: bool,
    translate: String,
    enhance: String,
    popup: String,
    terminal: String,
) -> std::result::Result<(), String> {
    crate::register_shortcuts(&app, enabled, &translate, &enhance, &popup, &terminal)
}

#[tauri::command]
pub async fn get_logs(state: State<'_, Arc<AppState>>) -> Result<Vec<LogEntry>> {
    let logs = state.logs.lock().unwrap();
    Ok(logs.clone())
}

