use std::sync::Arc;

use tauri::State;

use crate::domain::error::Result;
use crate::domain::llm_config::{ChatMessage, LLMConfig};
use crate::domain::prompt::Prompt;
use crate::interfaces::http::{add_log, LogEntry};

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

