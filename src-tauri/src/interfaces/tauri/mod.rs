use crate::application::use_cases::enhance::EnhanceUseCase;
use crate::application::use_cases::qa_ai::{ExploreResult, QaAiUseCase};
use crate::application::use_cases::qa_api_call::QaApiCallUseCase;
use crate::application::use_cases::qa_event::QaEventUseCase;
use crate::application::use_cases::qa_run::QaRunUseCase;
use crate::application::use_cases::qa_session::QaSessionUseCase;
use crate::application::use_cases::rag_ingestion::RagIngestionUseCase;
use crate::application::use_cases::retrieval_service::RetrievalService;
use crate::application::use_cases::translate::TranslateUseCase;
use crate::application::use_cases::typegen::TypeGenUseCase;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
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
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tracing::error;
use uuid::Uuid;

use crate::infrastructure::config::ConfigService;
use crate::infrastructure::llm_clients::LLMClient;
use crate::interfaces::http::{add_log, add_log_entry, LogEntry};
use crate::interfaces::mock_server::{
    build_status as build_mock_status, save_config as save_mock_server_config,
    start_mock_server, stop_mock_server, MockServerConfig, MockServerState, MockServerStatus,
};
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::rag_config::{SharedConfigManager, SharedFeedbackCollector};
use crate::application::use_cases::rag_metrics::{SharedMetricsCollector, SharedExperimentManager};
use crate::application::use_cases::rag_analytics::SharedAnalyticsLogger;
use crate::application::use_cases::conversation_service::ConversationService;

pub mod rag_commands;
use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JsonValue};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::Mutex as AsyncMutex;

pub struct AppState {
    pub translate_use_case: TranslateUseCase,
    pub enhance_use_case: EnhanceUseCase,
    pub typegen_use_case: TypeGenUseCase,
    pub qa_session_use_case: QaSessionUseCase,
    pub qa_event_use_case: QaEventUseCase,
    pub qa_ai_use_case: QaAiUseCase,
    pub qa_run_use_case: QaRunUseCase,
    pub qa_api_call_use_case: QaApiCallUseCase,
    pub rag_ingestion_use_case: RagIngestionUseCase,
    pub retrieval_service: Arc<RetrievalService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub qa_session_id: Mutex<Option<String>>,
    pub qa_recorder: Mutex<Option<QaRecorderHandle>>,
    pub repository: Arc<SqliteRepository>,
    pub rag_repository: Arc<RagRepository>,
    pub config_service: ConfigService,
    pub llm_client: Arc<dyn LLMClient + Send + Sync>,
    pub mock_server: Arc<MockServerState>,
    pub last_config: Mutex<LLMConfig>,
    pub preferred_source: Mutex<String>,
    pub preferred_target: Mutex<String>,
    pub logs: Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    /// RAG metrics collector for performance tracking
    pub metrics_collector: SharedMetricsCollector,
    /// A/B experiment manager for RAG experiments
    pub experiment_manager: SharedExperimentManager,
    /// Analytics logger for RAG operations
    pub analytics_logger: SharedAnalyticsLogger,
    /// RAG configuration manager
    pub config_manager: SharedConfigManager,
    /// User feedback collector
    pub feedback_collector: SharedFeedbackCollector,
    /// Conversation service for chat persistence
    pub conversation_service: Arc<ConversationService>,
}

const QA_EVENT_EMIT: &str = "qa-event-recorded";
const QA_RUN_STREAM_EMIT: &str = "qa-run-stream";
const QA_RUN_UPDATED_EMIT: &str = "qa-run-updated";
const QA_LOG_EMIT: &str = "qa-log";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct QaLogContext {
    session_id: Option<String>,
    run_id: Option<String>,
    run_type: Option<String>,
    mode: Option<String>,
    event_type: Option<String>,
    status_code: Option<i64>,
    latency_ms: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct QaLogEvent {
    time: String,
    level: String,
    source: String,
    message: String,
    status: String,
    error: Option<String>,
    context: Option<QaLogContext>,
}

pub(crate) struct QaRecorderHandle {
    child: Arc<AsyncMutex<Child>>,
    session_id: String,
    run_id: String,
    mode: String,
}

fn emit_status_log(
    app: &tauri::AppHandle,
    logs: &Arc<Mutex<Vec<LogEntry>>>,
    level: &str,
    source: &str,
    message: &str,
    status: &str,
    error: Option<&str>,
    context: Option<QaLogContext>,
) {
    let entry = add_log_entry(logs, level, source, message);
    let payload = QaLogEvent {
        time: entry.time,
        level: entry.level,
        source: entry.source,
        message: entry.message,
        status: status.to_string(),
        error: error.map(|value| value.to_string()),
        context,
    };
    if let Err(err) = app.emit(QA_LOG_EMIT, payload) {
        add_log(
            logs,
            "ERROR",
            "QA",
            &format!("Failed to emit QA log event: {}", err),
        );
    }
}

async fn record_recorder_event(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    session_id: &str,
    run_id: &str,
    mode: &str,
    payload: RecorderEventPayload,
) -> Result<()> {
    let event_type = payload.event_type.to_lowercase();
    if !matches!(event_type.as_str(), "click" | "input" | "submit") {
        return Ok(());
    }

    let previous_event = state
        .qa_event_use_case
        .latest_event_summary(session_id)
        .await?;

    let event_input = QaEventInput {
        event_type: event_type.clone(),
        selector: payload.selector.clone(),
        element_text: payload.element_text.clone(),
        value: payload.value.clone(),
        url: payload.url.clone(),
        meta_json: payload.meta.map(|meta| meta.to_string()),
        run_id: Some(run_id.to_string()),
        checkpoint_id: None,
        origin: Some(
            payload
                .origin
                .clone()
                .unwrap_or_else(|| if mode == "ai" { "ai" } else { "user" }.to_string()),
        ),
        recording_mode: Some("browser".to_string()),
    };

    let recorded = state
        .qa_event_use_case
        .record_event(session_id, event_input)
        .await?;

    if let Some(data_url) = payload.screenshot_data_url.as_deref() {
        add_log(
            &state.logs,
            "INFO",
            "QA",
            &format!(
                "QA recorder screenshot received (session_id={} event_id={})",
                session_id, recorded.id
            ),
        );
        if let Err(err) =
            persist_screenshot_data_url(app, state, session_id, data_url, Some(&recorded.id)).await
        {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to save recorder screenshot (session_id={} event_id={}): {}",
                    session_id, recorded.id, err
                ),
            );
        }
    }

    if let Err(err) = app.emit(QA_EVENT_EMIT, &recorded) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit QA event: {}", err),
        );
    }

    let stream_input = QaRunStreamInput {
        channel: "browser".to_string(),
        level: "info".to_string(),
        message: format!("Event recorded: {}", recorded.event_type),
        payload_json: serde_json::to_string(&recorded).ok(),
    };
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(run_id, stream_input)
        .await
    {
        if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stream_event) {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to emit run stream: {}", err),
            );
        }
    }

    if let Ok(checkpoints) = state
        .qa_ai_use_case
        .maybe_create_checkpoint_from_event(session_id, &recorded, previous_event)
        .await
    {
        for checkpoint in checkpoints {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "Checkpoint created: id={} seq={} events={}..{}",
                    checkpoint.id,
                    checkpoint.seq,
                    checkpoint.start_event_seq,
                    checkpoint.end_event_seq
                ),
            );
        }
    }

    Ok(())
}

async fn record_recorder_network(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    session_id: &str,
    run_id: &str,
    payload: RecorderNetworkPayload,
) -> Result<()> {
    let request_headers_json = payload
        .request_headers
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok());
    let response_headers_json = payload
        .response_headers
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok());
    let method = payload.method.clone();
    let url = payload.url.clone();

    let _call = state
        .qa_api_call_use_case
        .record_api_call(
            session_id,
            run_id,
            &method,
            &url,
            request_headers_json,
            payload.request_body.clone(),
            payload.status,
            response_headers_json,
            payload.response_body.clone(),
            payload.timing_ms,
        )
        .await?;

    let meta_json = serde_json::json!({
        "method": method,
        "url": url,
        "status": payload.status,
        "timing_ms": payload.timing_ms
    })
    .to_string();
    let event_input = QaEventInput {
        event_type: "api_response".to_string(),
        selector: None,
        element_text: None,
        value: None,
        url: Some(payload.url.clone()),
        meta_json: Some(meta_json),
        run_id: Some(run_id.to_string()),
        checkpoint_id: None,
        origin: Some("system".to_string()),
        recording_mode: Some("browser".to_string()),
    };
    let recorded = state
        .qa_event_use_case
        .record_event(session_id, event_input)
        .await?;

    let stream_input = QaRunStreamInput {
        channel: "api".to_string(),
        level: "info".to_string(),
        message: format!(
            "Network response: {} {}",
            method,
            payload.status.unwrap_or_default()
        ),
        payload_json: serde_json::to_string(&recorded).ok(),
    };
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(run_id, stream_input)
        .await
    {
        if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stream_event) {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to emit run stream: {}", err),
            );
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaScreenshotResult {
    pub path: String,
    pub event_id: String,
    pub artifact_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiKeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiFormField {
    pub key: String,
    pub value: Option<String>,
    pub file_name: Option<String>,
    pub file_base64: Option<String>,
    pub content_type: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<QaApiKeyValue>,
    pub query_params: Vec<QaApiKeyValue>,
    pub body_type: Option<String>,
    pub body_json: Option<String>,
    pub form_data: Vec<QaApiFormField>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaBrowserReplayEvent {
    pub event_type: String,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub url: Option<String>,
    pub ts: i64,
    pub seq: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaBrowserReplayPayload {
    pub target_url: String,
    pub events: Vec<QaBrowserReplayEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiResponse {
    pub status: u16,
    pub duration_ms: i64,
    pub headers: Vec<QaApiKeyValue>,
    pub body: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecorderMessage {
    #[serde(rename = "type")]
    kind: String,
    payload: JsonValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecorderEventPayload {
    event_type: String,
    selector: Option<String>,
    element_text: Option<String>,
    value: Option<String>,
    url: Option<String>,
    meta: Option<JsonValue>,
    origin: Option<String>,
    screenshot_data_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecorderNetworkPayload {
    method: String,
    url: String,
    status: Option<i64>,
    timing_ms: Option<i64>,
    request_headers: Option<JsonValue>,
    response_headers: Option<JsonValue>,
    request_body: Option<String>,
    response_body: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecorderStatusPayload {
    level: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecorderAuthPayload {
    path: String,
}

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

#[tauri::command]
pub async fn mock_server_get_config(state: State<'_, Arc<AppState>>) -> Result<MockServerConfig> {
    add_log(&state.logs, "INFO", "MockServer", "Mock server config requested");
    let config = state.mock_server.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
pub async fn mock_server_update_config(
    state: State<'_, Arc<AppState>>,
    config: MockServerConfig,
) -> Result<MockServerConfig> {
    add_log(&state.logs, "INFO", "MockServer", "Mock server config updated");
    let mut current = state.mock_server.config.lock().unwrap();
    *current = config.clone();
    save_mock_server_config(&state.mock_server)?;
    Ok(config)
}

#[tauri::command]
pub async fn mock_server_start(state: State<'_, Arc<AppState>>) -> Result<MockServerStatus> {
    add_log(&state.logs, "INFO", "MockServer", "Mock server start requested");
    start_mock_server(state.mock_server.clone()).await?;
    Ok(build_mock_status(&state.mock_server))
}

#[tauri::command]
pub async fn mock_server_stop(state: State<'_, Arc<AppState>>) -> Result<MockServerStatus> {
    add_log(&state.logs, "INFO", "MockServer", "Mock server stop requested");
    stop_mock_server(state.mock_server.clone()).await?;
    Ok(build_mock_status(&state.mock_server))
}

#[tauri::command]
pub async fn mock_server_status(state: State<'_, Arc<AppState>>) -> Result<MockServerStatus> {
    add_log(&state.logs, "INFO", "MockServer", "Mock server status requested");
    Ok(build_mock_status(&state.mock_server))
}

#[tauri::command]
pub async fn qa_start_session(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    title: String,
    goal: String,
    session_type: String,
    is_positive_case: bool,
    target_url: Option<String>,
    api_base_url: Option<String>,
    auth_profile_json: Option<String>,
    source_session_id: Option<String>,
    notes: Option<String>,
) -> Result<QaSession> {
    add_log(&state.logs, "INFO", "QA", "QA start session requested");
    if state.qa_session_id.lock().unwrap().is_some() {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "Failed to start QA session: session already active",
        );
        emit_status_log(
            &app,
            &state.logs,
            "ERROR",
            "QA",
            "Failed to start QA session",
            "failed",
            Some("A QA session is already active."),
            None,
        );
        return Err(AppError::ValidationError(
            "A QA session is already active.".to_string(),
        ));
    }

    let app_version = Some(app.package_info().version.to_string());
    let os = Some(std::env::consts::OS.to_string());

    let session = match state
        .qa_session_use_case
        .start_session(
            title,
            goal,
            session_type,
            is_positive_case,
            target_url,
            api_base_url,
            auth_profile_json,
            source_session_id,
            app_version,
            os,
            notes,
        )
        .await
    {
        Ok(session) => session,
        Err(err) => {
            error!(error = %err, "Failed to start QA session");
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to start QA session",
                "failed",
                Some(&err.to_string()),
                None,
            );
            return Err(err);
        }
    };

    *state.qa_session_id.lock().unwrap() = Some(session.id.clone());
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA session started: id={} title=\"{}\" type={} positive_case={}",
            session.id, session.title, session.session_type, session.is_positive_case
        ),
    );
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA session started",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(session.id.clone()),
            run_id: None,
            run_type: None,
            mode: Some(session.session_type.clone()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );

    Ok(session)
}

#[tauri::command]
pub async fn qa_end_session(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: Option<String>,
) -> Result<QaSession> {
    add_log(&state.logs, "INFO", "QA", "QA end session requested");
    let active_id = {
        let current = state.qa_session_id.lock().unwrap();
        session_id.or_else(|| current.clone())
    };

    let session_id = active_id.ok_or_else(|| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "Failed to end QA session: no active session",
        );
        emit_status_log(
            &app,
            &state.logs,
            "ERROR",
            "QA",
            "Failed to end QA session",
            "failed",
            Some("No active QA session to stop."),
            None,
        );
        AppError::ValidationError("No active QA session to stop.".to_string())
    })?;

    let session = match state.qa_session_use_case.end_session(&session_id).await {
        Ok(session) => session,
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to end QA session");
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to end QA session",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: None,
                    mode: None,
                    event_type: None,
                    status_code: None,
                    latency_ms: None,
                }),
            );
            return Err(err);
        }
    };
    *state.qa_session_id.lock().unwrap() = None;
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA session ended: id={}", session.id),
    );
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA session ended",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(session.id.clone()),
            run_id: None,
            run_type: None,
            mode: Some(session.session_type.clone()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );

    Ok(session)
}

#[tauri::command]
pub async fn qa_start_run(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    run_type: String,
    mode: String,
    triggered_by: String,
    source_run_id: Option<String>,
    checkpoint_id: Option<String>,
    meta_json: Option<String>,
) -> Result<QaSessionRun> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA start run requested (session_id={}, type={})",
            session_id, run_type
        ),
    );
    let run = match state
        .qa_run_use_case
        .start_run(
            &session_id,
            &run_type,
            &mode,
            &triggered_by,
            source_run_id,
            checkpoint_id,
            meta_json,
        )
        .await
    {
        Ok(run) => run,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to start QA run",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: Some(run_type.clone()),
                    mode: Some(mode.clone()),
                    event_type: None,
                    status_code: None,
                    latency_ms: None,
                }),
            );
            return Err(err);
        }
    };
    if let Err(err) = app.emit(QA_RUN_UPDATED_EMIT, &run) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit run update: {}", err),
        );
    }
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(
            &run.id,
            QaRunStreamInput {
                channel: "system".to_string(),
                level: "info".to_string(),
                message: format!("Run started ({})", run.run_type),
                payload_json: None,
            },
        )
        .await
    {
        if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stream_event) {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to emit run stream: {}", err),
            );
        }
    }
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA run started",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(run.session_id.clone()),
            run_id: Some(run.id.clone()),
            run_type: Some(run.run_type.clone()),
            mode: Some(run.mode.clone()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );
    Ok(run)
}

#[tauri::command]
pub async fn qa_end_run(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
    status: String,
) -> Result<QaSessionRun> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA end run requested (run_id={}, status={})",
            run_id, status
        ),
    );
    let run = match state.qa_run_use_case.end_run(&run_id, &status).await {
        Ok(run) => run,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to end QA run",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: None,
                    run_id: Some(run_id.clone()),
                    run_type: None,
                    mode: None,
                    event_type: None,
                    status_code: None,
                    latency_ms: None,
                }),
            );
            return Err(err);
        }
    };
    if let Err(err) = app.emit(QA_RUN_UPDATED_EMIT, &run) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit run update: {}", err),
        );
    }
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(
            &run.id,
            QaRunStreamInput {
                channel: "system".to_string(),
                level: "info".to_string(),
                message: format!("Run ended ({})", run.status),
                payload_json: None,
            },
        )
        .await
    {
        if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stream_event) {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to emit run stream: {}", err),
            );
        }
    }
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA run ended",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(run.session_id.clone()),
            run_id: Some(run.id.clone()),
            run_type: Some(run.run_type.clone()),
            mode: Some(run.mode.clone()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );
    Ok(run)
}

#[tauri::command]
pub async fn qa_start_browser_recorder(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    run_id: String,
    target_url: String,
    mode: String,
    screenshot_delay_ms: Option<u64>,
    event_interval_ms: Option<u64>,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        "QA start browser recorder requested",
    );
    let session_id = session_id.trim().to_string();
    let run_id = run_id.trim().to_string();
    let target_url = target_url.trim().to_string();
    let mode = mode.trim().to_string();
    let run_type = if mode == "ai" {
        "ai_explore".to_string()
    } else {
        "record".to_string()
    };

    if session_id.is_empty() || run_id.is_empty() || target_url.is_empty() {
        return Err(AppError::ValidationError(
            "Session, run, and target URL are required.".to_string(),
        ));
    }
    if mode != "manual" && mode != "ai" {
        return Err(AppError::ValidationError(
            "Recorder mode must be 'manual' or 'ai'.".to_string(),
        ));
    }

    let mut recorder_guard = state.qa_recorder.lock().unwrap();
    if recorder_guard.is_some() {
        return Err(AppError::ValidationError(
            "A browser recorder is already running.".to_string(),
        ));
    }

    let app_data_dir = resolve_app_data_dir(&app)
        .map_err(|err| AppError::Internal(format!("Failed to resolve app data dir: {}", err)))?;
    let qa_sessions_dir = ensure_qa_sessions_root(&app_data_dir)
        .map_err(|err| AppError::Internal(format!("Failed to ensure QA session root: {}", err)))?;
    let session_dir = ensure_session_dir(&qa_sessions_dir, &session_id)
        .map_err(|err| AppError::Internal(format!("Failed to ensure QA session dir: {}", err)))?;
    let storage_state_path = session_dir.join(format!("storage_state_{}.json", run_id));

    let current_dir = std::env::current_dir()
        .map_err(|err| AppError::Internal(format!("Failed to resolve cwd: {}", err)))?;
    let script_candidates = [
        current_dir.join("scripts/qa-browser-recorder.mjs"),
        current_dir
            .join("..")
            .join("scripts/qa-browser-recorder.mjs"),
    ];
    let script_path = script_candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| AppError::NotFound("QA recorder script not found.".to_string()))?;

    let mut command = TokioCommand::new("node");
    command
        .arg(script_path)
        .arg("--url")
        .arg(&target_url)
        .arg("--mode")
        .arg(&mode)
        .arg("--storage")
        .arg(&storage_state_path);
    if let Some(delay_ms) = screenshot_delay_ms {
        command.arg("--screenshot-delay").arg(delay_ms.to_string());
    }
    if let Some(interval_ms) = event_interval_ms {
        command.arg("--event-interval").arg(interval_ms.to_string());
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| AppError::Internal(format!("Failed to launch recorder: {}", err)))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Internal("Recorder stdout unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Internal("Recorder stderr unavailable".to_string()))?;

    let child = Arc::new(AsyncMutex::new(child));
    *recorder_guard = Some(QaRecorderHandle {
        child: child.clone(),
        session_id: session_id.clone(),
        run_id: run_id.clone(),
        mode: mode.clone(),
    });

    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    let session_clone = session_id.clone();
    let run_clone = run_id.clone();
    let mode_clone = mode.clone();
    let run_type_clone = run_type.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = TokioBufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            match serde_json::from_str::<RecorderMessage>(&line) {
                Ok(message) => match message.kind.as_str() {
                    "event" => {
                        if let Ok(payload) =
                            serde_json::from_value::<RecorderEventPayload>(message.payload)
                        {
                            if let Err(err) = record_recorder_event(
                                &app_clone,
                                &state_clone,
                                &session_clone,
                                &run_clone,
                                &mode_clone,
                                payload,
                            )
                            .await
                            {
                                add_log(
                                    &state_clone.logs,
                                    "ERROR",
                                    "QA",
                                    &format!("Recorder event failed: {}", err),
                                );
                            }
                        }
                    }
                    "network" => {
                        if let Ok(payload) =
                            serde_json::from_value::<RecorderNetworkPayload>(message.payload)
                        {
                            if let Err(err) = record_recorder_network(
                                &app_clone,
                                &state_clone,
                                &session_clone,
                                &run_clone,
                                payload,
                            )
                            .await
                            {
                                add_log(
                                    &state_clone.logs,
                                    "ERROR",
                                    "QA",
                                    &format!("Recorder network failed: {}", err),
                                );
                            }
                        }
                    }
                    "status" => {
                        if let Ok(payload) =
                            serde_json::from_value::<RecorderStatusPayload>(message.payload)
                        {
                            let status = if payload.level.to_lowercase() == "error" {
                                "failed"
                            } else {
                                "success"
                            };
                            emit_status_log(
                                &app_clone,
                                &state_clone.logs,
                                &payload.level.to_uppercase(),
                                "QA",
                                &payload.message,
                                status,
                                None,
                                Some(QaLogContext {
                                    session_id: Some(session_clone.clone()),
                                    run_id: Some(run_clone.clone()),
                                    run_type: Some(run_type_clone.clone()),
                                    mode: Some("browser".to_string()),
                                    event_type: None,
                                    status_code: None,
                                    latency_ms: None,
                                }),
                            );
                        }
                    }
                    "auth_state" => {
                        if let Ok(payload) =
                            serde_json::from_value::<RecorderAuthPayload>(message.payload)
                        {
                            emit_status_log(
                                &app_clone,
                                &state_clone.logs,
                                "INFO",
                                "QA",
                                "Auth state saved",
                                "success",
                                None,
                                Some(QaLogContext {
                                    session_id: Some(session_clone.clone()),
                                    run_id: Some(run_clone.clone()),
                                    run_type: Some(run_type_clone.clone()),
                                    mode: Some("browser".to_string()),
                                    event_type: Some(payload.path),
                                    status_code: None,
                                    latency_ms: None,
                                }),
                            );
                        }
                    }
                    _ => {
                        add_log(
                            &state_clone.logs,
                            "INFO",
                            "QA",
                            &format!("Recorder message: {}", line),
                        );
                    }
                },
                Err(_) => {
                    add_log(
                        &state_clone.logs,
                        "INFO",
                        "QA",
                        &format!("Recorder output: {}", line),
                    );
                }
            }
        }
    });

    let state_clone = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = TokioBufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            add_log(&state_clone.logs, "ERROR", "QA", &line);
        }
    });

    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    let run_clone = run_id.clone();
    let child_clone = child.clone();
    tauri::async_runtime::spawn(async move {
        let status = child_clone.lock().await.wait().await;
        let mut guard = state_clone.qa_recorder.lock().unwrap();
        guard.take();
        let message = match status {
            Ok(exit) if exit.success() => "Recorder stopped",
            Ok(exit) => {
                add_log(
                    &state_clone.logs,
                    "ERROR",
                    "QA",
                    &format!("Recorder exited with status: {}", exit),
                );
                "Recorder stopped with errors"
            }
            Err(err) => {
                add_log(
                    &state_clone.logs,
                    "ERROR",
                    "QA",
                    &format!("Recorder wait failed: {}", err),
                );
                "Recorder stopped with errors"
            }
        };
        emit_status_log(
            &app_clone,
            &state_clone.logs,
            "INFO",
            "QA",
            message,
            "success",
            None,
            Some(QaLogContext {
                session_id: None,
                run_id: Some(run_clone.clone()),
                run_type: None,
                mode: Some("browser".to_string()),
                event_type: None,
                status_code: None,
                latency_ms: None,
            }),
        );
    });

    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "Browser recorder started",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(session_id),
            run_id: Some(run_id),
            run_type: Some(run_type.to_string()),
            mode: Some("browser".to_string()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );

    Ok(())
}

#[tauri::command]
pub async fn qa_stop_browser_recorder(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: Option<String>,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        "QA stop browser recorder requested",
    );
    let handle = {
        let mut recorder_guard = state.qa_recorder.lock().unwrap();
        recorder_guard.take()
    };

    let handle = match handle {
        Some(handle) => handle,
        None => {
            return Err(AppError::ValidationError(
                "No active browser recorder.".to_string(),
            ));
        }
    };

    if let Some(expected) = run_id.as_ref() {
        if expected != &handle.run_id {
            return Err(AppError::ValidationError(
                "Recorder run id mismatch.".to_string(),
            ));
        }
    }

    {
        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "Browser recorder stopped",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(handle.session_id),
            run_id: Some(handle.run_id),
            run_type: None,
            mode: Some(handle.mode),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );

    Ok(())
}

#[tauri::command]
pub async fn qa_append_run_stream_event(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
    event: QaRunStreamInput,
) -> Result<QaRunStreamEvent> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA append run stream event (run_id={})", run_id),
    );
    let stored = state
        .qa_run_use_case
        .append_stream_event(&run_id, event)
        .await?;
    if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stored) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit run stream: {}", err),
        );
    }
    Ok(stored)
}

#[tauri::command]
pub async fn qa_list_run_stream_events(
    state: State<'_, Arc<AppState>>,
    run_id: String,
    limit: Option<i64>,
) -> Result<Vec<QaRunStreamEvent>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list run stream events (run_id={})", run_id),
    );
    state
        .qa_run_use_case
        .list_stream_events(&run_id, limit.unwrap_or(50))
        .await
}

#[tauri::command]
pub async fn qa_list_sessions(
    state: State<'_, Arc<AppState>>,
    limit: Option<i64>,
) -> Result<Vec<QaSession>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list sessions requested (limit={})", limit.unwrap_or(50)),
    );
    match state.qa_session_use_case.list_sessions(limit).await {
        Ok(sessions) => Ok(sessions),
        Err(err) => {
            error!(error = %err, "Failed to list QA sessions");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to list QA sessions: {}", err),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_get_session(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<QaSession> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA get session requested (id={})", session_id),
    );
    match state.qa_session_use_case.get_session(&session_id).await {
        Ok(session) => Ok(session),
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to fetch QA session");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to fetch QA session (id={}): {}", session_id, err),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_execute_api_request(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    request: QaApiRequest,
) -> Result<QaApiResponse> {
    let source = request
        .source
        .as_deref()
        .map(|value| format!("({}) ", value))
        .unwrap_or_default();

    let request_body_note = if let Some(body_json) = request.body_json.as_ref() {
        format!(" body_len={}", body_json.len())
    } else if !request.form_data.is_empty() {
        " body_len=form-data".to_string()
    } else {
        String::new()
    };
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "[Request] {source}{} {}{}",
            request.method, request.url, request_body_note
        ),
    );

    let method =
        Method::from_bytes(request.method.trim().to_uppercase().as_bytes()).map_err(|_| {
            AppError::ValidationError("Invalid HTTP method for API request.".to_string())
        })?;
    let mut url = url::Url::parse(request.url.trim())
        .map_err(|_| AppError::ValidationError("Invalid URL for API request.".to_string()))?;

    if !request.query_params.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for param in request.query_params.into_iter().filter(|item| item.enabled) {
            if param.key.trim().is_empty() {
                continue;
            }
            pairs.append_pair(param.key.trim(), param.value.trim());
        }
    }

    let mut header_map = HeaderMap::new();
    for header in request.headers.into_iter().filter(|item| item.enabled) {
        if header.key.trim().is_empty() {
            continue;
        }
        let name = HeaderName::from_bytes(header.key.trim().as_bytes()).map_err(|_| {
            AppError::ValidationError("Invalid header name for API request.".to_string())
        })?;
        let value = HeaderValue::from_str(header.value.trim()).map_err(|_| {
            AppError::ValidationError("Invalid header value for API request.".to_string())
        })?;
        header_map.insert(name, value);
    }

    let client = reqwest::Client::new();
    let mut builder = client
        .request(method, url.clone())
        .headers(header_map.clone());

    if let Some(body_type) = request.body_type.as_deref() {
        if body_type == "json" {
            if let Some(body_json) = request.body_json.as_ref() {
                if !body_json.trim().is_empty() {
                    if !header_map.contains_key(reqwest::header::CONTENT_TYPE) {
                        builder = builder.header(reqwest::header::CONTENT_TYPE, "application/json");
                    }
                    builder = builder.body(body_json.trim().to_string());
                }
            }
        } else if body_type == "form" {
            let mut form = Form::new();
            for field in request.form_data.into_iter().filter(|item| item.enabled) {
                if field.key.trim().is_empty() {
                    continue;
                }
                if let Some(file_base64) = field.file_base64.as_ref() {
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(file_base64)
                        .map_err(|_| {
                            AppError::ValidationError(
                                "Failed to decode form-data file payload.".to_string(),
                            )
                        })?;
                    let mut part = Part::bytes(decoded).file_name(
                        field
                            .file_name
                            .clone()
                            .unwrap_or_else(|| "upload".to_string()),
                    );
                    if let Some(content_type) = field.content_type.as_ref() {
                        part = part.mime_str(content_type).map_err(|_| {
                            AppError::ValidationError("Invalid form-data content type.".to_string())
                        })?;
                    }
                    form = form.part(field.key.trim().to_string(), part);
                } else if let Some(value) = field.value.as_ref() {
                    form = form.text(field.key.trim().to_string(), value.clone());
                }
            }
            builder = builder.multipart(form);
        }
    }

    let start = std::time::Instant::now();
    let response = match builder.send().await {
        Ok(response) => response,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                &format!("QA API request failed: {} {}", request.method, request.url),
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: None,
                    run_id: None,
                    run_type: None,
                    mode: Some("api".to_string()),
                    event_type: Some("api_request".to_string()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            return Err(AppError::Internal(format!("API request failed: {}", err)));
        }
    };

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let mut headers = Vec::new();
    for (key, value) in response.headers().iter() {
        if let Ok(value_str) = value.to_str() {
            headers.push(QaApiKeyValue {
                key: key.to_string(),
                value: value_str.to_string(),
                enabled: true,
            });
        }
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "QA API response read failed",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: None,
                    run_id: None,
                    run_type: None,
                    mode: Some("api".to_string()),
                    event_type: Some("api_response".to_string()),
                    status_code: Some(status as i64),
                    latency_ms: Some(start.elapsed().as_millis() as i64),
                }),
            );
            return Err(AppError::Internal(
                "Failed to read API response body.".to_string(),
            ));
        }
    };

    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "[Response] {source}status={} body_len={}",
            status,
            body.len()
        ),
    );
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA API response received",
        "success",
        None,
        Some(QaLogContext {
            session_id: None,
            run_id: None,
            run_type: None,
            mode: Some("api".to_string()),
            event_type: Some("api_response".to_string()),
            status_code: Some(status as i64),
            latency_ms: Some(start.elapsed().as_millis() as i64),
        }),
    );

    Ok(QaApiResponse {
        status,
        duration_ms: start.elapsed().as_millis() as i64,
        headers,
        body,
        content_type,
    })
}

#[tauri::command]
pub async fn qa_replay_browser(
    state: State<'_, Arc<AppState>>,
    target_url: String,
    events: Vec<QaBrowserReplayEvent>,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA browser replay requested (events={}, url={})",
            events.len(),
            target_url
        ),
    );

    let payload = QaBrowserReplayPayload { target_url, events };
    let payload_json = serde_json::to_string(&payload)
        .map_err(|err| AppError::Internal(format!("Replay payload failed: {}", err)))?;

    let temp_path = std::env::temp_dir().join(format!("qa_browser_replay_{}.json", Uuid::new_v4()));
    fs::write(&temp_path, payload_json)
        .map_err(|err| AppError::Internal(format!("Failed to write replay payload: {}", err)))?;

    let current_dir = std::env::current_dir()
        .map_err(|err| AppError::Internal(format!("Failed to resolve cwd: {}", err)))?;
    let script_candidates = [
        current_dir.join("scripts/qa-browser-replay.mjs"),
        current_dir.join("..").join("scripts/qa-browser-replay.mjs"),
    ];
    let script_path = script_candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| AppError::NotFound("QA replay script not found.".to_string()))?;

    let logs = state.logs.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let mut child = StdCommand::new("node")
            .arg(script_path)
            .arg(&temp_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| AppError::Internal(format!("Failed to launch replay: {}", err)))?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                add_log(&logs, "INFO", "QA", &format!("QA replay: {}", line));
            }
        }
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                add_log(&logs, "ERROR", "QA", &format!("QA replay error: {}", line));
            }
        }

        let status = child
            .wait()
            .map_err(|err| AppError::Internal(format!("Replay failed: {}", err)))?;
        if !status.success() {
            return Err(AppError::Internal(
                "QA browser replay process failed.".to_string(),
            ));
        }
        add_log(&logs, "INFO", "QA", "QA browser replay completed");
        Ok(())
    })
    .await
    .map_err(|err| AppError::Internal(format!("Replay task failed: {}", err)))??;

    Ok(())
}

#[tauri::command]
pub async fn qa_record_event(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    event: QaEventInput,
    session_id: Option<String>,
) -> Result<QaEvent> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA record event requested (type={})", event.event_type),
    );
    let active_id = {
        let current = state.qa_session_id.lock().unwrap();
        session_id.or_else(|| current.clone())
    };

    let session_id = active_id.ok_or_else(|| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "Failed to record QA event: no active session",
        );
        AppError::ValidationError("No active QA session for event.".to_string())
    })?;

    let previous_event = match state
        .qa_event_use_case
        .latest_event_summary(&session_id)
        .await
    {
        Ok(summary) => summary,
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to read latest event summary (session_id={}): {}",
                    session_id, err
                ),
            );
            return Err(err);
        }
    };

    let event_context = QaLogContext {
        session_id: Some(session_id.clone()),
        run_id: event.run_id.clone(),
        run_type: None,
        mode: event.recording_mode.clone(),
        event_type: Some(event.event_type.clone()),
        status_code: None,
        latency_ms: None,
    };

    match state
        .qa_event_use_case
        .record_event(&session_id, event)
        .await
    {
        Ok(recorded) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA event recorded: type={} selector={} url={}",
                    recorded.event_type,
                    recorded.selector.as_deref().unwrap_or("-"),
                    recorded.url.as_deref().unwrap_or("-")
                ),
            );
            emit_status_log(
                &app,
                &state.logs,
                "INFO",
                "QA",
                "QA event recorded",
                "success",
                None,
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: recorded.run_id.clone(),
                    run_type: None,
                    mode: recorded.recording_mode.clone(),
                    event_type: Some(recorded.event_type.clone()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            if let Err(err) = app.emit(QA_EVENT_EMIT, &recorded) {
                add_log(
                    &state.logs,
                    "ERROR",
                    "QA",
                    &format!("Failed to emit QA event: {}", err),
                );
            }
            if let Some(run_id) = recorded.run_id.as_ref() {
                let channel = if recorded.event_type.contains("api")
                    || recorded.event_type.contains("curl")
                {
                    "api"
                } else {
                    "browser"
                };
                let stream_input = QaRunStreamInput {
                    channel: channel.to_string(),
                    level: "info".to_string(),
                    message: format!("Event recorded: {}", recorded.event_type),
                    payload_json: serde_json::to_string(&recorded).ok(),
                };
                match state
                    .qa_run_use_case
                    .append_stream_event(run_id, stream_input)
                    .await
                {
                    Ok(stream_event) => {
                        if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stream_event) {
                            add_log(
                                &state.logs,
                                "ERROR",
                                "QA",
                                &format!("Failed to emit run stream: {}", err),
                            );
                        }
                    }
                    Err(err) => {
                        add_log(
                            &state.logs,
                            "ERROR",
                            "QA",
                            &format!(
                                "Failed to append run stream event (run_id={}): {}",
                                run_id, err
                            ),
                        );
                    }
                }
            }
            match state
                .qa_ai_use_case
                .maybe_create_checkpoint_from_event(&session_id, &recorded, previous_event)
                .await
            {
                Ok(checkpoints) => {
                    for checkpoint in checkpoints {
                        add_log(
                            &state.logs,
                            "INFO",
                            "QA",
                            &format!(
                                "Checkpoint created: id={} seq={} events={}..{}",
                                checkpoint.id,
                                checkpoint.seq,
                                checkpoint.start_event_seq,
                                checkpoint.end_event_seq
                            ),
                        );
                    }
                }
                Err(err) => {
                    add_log(
                        &state.logs,
                        "ERROR",
                        "QA",
                        &format!(
                            "Checkpoint creation failed (session_id={}): {}",
                            session_id, err
                        ),
                    );
                }
            }
            Ok(recorded)
        }
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to record QA event");
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to record QA event",
                "failed",
                Some(&err.to_string()),
                Some(event_context),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_open_devtools(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<()> {
    add_log(&state.logs, "INFO", "QA", "QA open devtools requested");
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(any(debug_assertions, feature = "devtools"))]
        {
            window.open_devtools();
            add_log(&state.logs, "INFO", "QA", "QA devtools opened");
        }
        #[cfg(not(any(debug_assertions, feature = "devtools")))]
        {
            add_log(
                &state.logs,
                "WARN",
                "QA",
                "QA devtools unavailable (devtools feature disabled)",
            );
        }
        Ok(())
    } else {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "QA devtools failed: main window not found",
        );
        Err(AppError::NotFound("Main window not found.".to_string()))
    }
}

#[tauri::command]
pub async fn qa_list_events(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<QaEvent>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list events requested (session_id={})", session_id),
    );
    match state.qa_event_use_case.list_events(&session_id).await {
        Ok(events) => Ok(events),
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to list QA events");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list QA events (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_list_screenshots(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<QaEvent>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list screenshots requested (session_id={})", session_id),
    );
    match state.qa_event_use_case.list_screenshots(&session_id).await {
        Ok(events) => Ok(events),
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to list QA screenshots");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list QA screenshots (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

async fn persist_screenshot_data_url(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    session_id: &str,
    data_url: &str,
    event_id: Option<&str>,
) -> Result<QaScreenshotResult> {
    let (mime, bytes) = match decode_data_url(data_url) {
        Ok(result) => result,
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to decode screenshot data: {}", err),
            );
            return Err(AppError::ValidationError(err));
        }
    };

    let app_data_dir = resolve_app_data_dir(app).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve app data dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;
    let qa_sessions_dir = ensure_qa_sessions_root(&app_data_dir).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve QA sessions dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;
    let session_dir = ensure_session_dir(&qa_sessions_dir, session_id).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve QA session dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;
    let screenshots_dir = ensure_session_screenshots_dir(&session_dir).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve QA screenshots dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;

    let now = chrono::Utc::now().timestamp_millis();
    let artifact_id = Uuid::new_v4().to_string();
    let filename = format!("screenshot_{}_{}.png", now, &artifact_id[..8]);
    let path = screenshots_dir.join(filename);
    std::fs::write(&path, bytes).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to save screenshot: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;

    let resolved_event_id = match state
        .qa_event_use_case
        .attach_screenshot(
            session_id,
            event_id.map(|value| value.to_string()),
            &artifact_id,
            path.to_string_lossy().as_ref(),
            Some(&mime),
            None,
            None,
            now,
        )
        .await
    {
        Ok(event_id) => event_id,
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to attach screenshot");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to attach screenshot (session_id={}): {}",
                    session_id, err
                ),
            );
            return Err(err);
        }
    };

    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA screenshot saved: session_id={} event_id={} path={}",
            session_id,
            resolved_event_id,
            path.display()
        ),
    );

    Ok(QaScreenshotResult {
        path: path.to_string_lossy().to_string(),
        event_id: resolved_event_id,
        artifact_id,
    })
}

#[tauri::command]
pub async fn qa_capture_screenshot(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    data_url: String,
    event_id: Option<String>,
) -> Result<QaScreenshotResult> {
    let session_id = session_id.trim().to_string();
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA capture screenshot requested (session_id={} event_id={} bytes={})",
            session_id,
            event_id.as_deref().unwrap_or("-"),
            data_url.len()
        ),
    );
    if session_id.is_empty() {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "Failed to capture screenshot: session id missing",
        );
        return Err(AppError::ValidationError(
            "Session id is required.".to_string(),
        ));
    }

    persist_screenshot_data_url(&app, &state, &session_id, &data_url, event_id.as_deref()).await
}

fn decode_data_url(data_url: &str) -> std::result::Result<(String, Vec<u8>), String> {
    let (header, data) = data_url
        .split_once(',')
        .ok_or_else(|| "Screenshot data is not a valid data URL.".to_string())?;

    if !header.starts_with("data:") || !header.contains(";base64") {
        return Err("Screenshot data is not base64 encoded.".to_string());
    }

    let mime = header
        .trim_start_matches("data:")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_string();

    let trimmed = data.trim().trim_start_matches('\u{feff}');
    let cleaned = trimmed.replace('\n', "").replace('\r', "").replace(' ', "");

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .or_else(|_| {
            let url_safe = cleaned.replace('-', "+").replace('_', "/");
            base64::engine::general_purpose::STANDARD.decode(url_safe.as_bytes())
        })
        .map_err(|e| format!("Failed to decode screenshot payload: {e}"))?;

    Ok((mime, bytes))
}

#[tauri::command]
pub async fn qa_list_events_page(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    page: i64,
    page_size: i64,
) -> Result<QaEventPage> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA list events page requested (session_id={} page={} page_size={})",
            session_id, page, page_size
        ),
    );
    match state
        .qa_event_use_case
        .list_events_page(&session_id, page, page_size)
        .await
    {
        Ok(events_page) => Ok(events_page),
        Err(err) => {
            error!(
                error = %err,
                session_id = %session_id,
                "Failed to list QA events page"
            );
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list QA events page (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_delete_events(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    event_ids: Vec<String>,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA delete events requested (session_id={} count={})",
            session_id,
            event_ids.len()
        ),
    );
    match state
        .qa_event_use_case
        .delete_events(&session_id, event_ids)
        .await
    {
        Ok(deleted) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA delete events success (session_id={} deleted={})",
                    session_id, deleted
                ),
            );
            Ok(deleted)
        }
        Err(err) => {
            error!(
                error = %err,
                session_id = %session_id,
                "Failed to delete QA events"
            );
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to delete QA events (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_delete_session(state: State<'_, Arc<AppState>>, session_id: String) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA delete session requested (session_id={})", session_id),
    );
    match state.qa_session_use_case.delete_session(&session_id).await {
        Ok(deleted) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA delete session success (session_id={} deleted={})",
                    session_id, deleted
                ),
            );
            Ok(deleted)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to delete session (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_create_checkpoint(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    title: Option<String>,
) -> Result<QaCheckpoint> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA create checkpoint requested (session_id={})", session_id),
    );
    match state
        .qa_ai_use_case
        .create_checkpoint(&session_id, title)
        .await
    {
        Ok(checkpoint) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA checkpoint created: id={} seq={} events={}..{}",
                    checkpoint.id,
                    checkpoint.seq,
                    checkpoint.start_event_seq,
                    checkpoint.end_event_seq
                ),
            );
            Ok(checkpoint)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to create checkpoint (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_list_checkpoints(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<QaCheckpoint>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list checkpoints requested (session_id={})", session_id),
    );
    match state.qa_ai_use_case.list_checkpoints(&session_id).await {
        Ok(checkpoints) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA list checkpoints success (session_id={} count={})",
                    session_id,
                    checkpoints.len()
                ),
            );
            Ok(checkpoints)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list checkpoints (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_generate_checkpoint_summary(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    checkpoint_id: String,
    config: LLMConfig,
    output_language: String,
) -> Result<QaCheckpointSummary> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA generate summary requested (session_id={} checkpoint_id={} model={} language={})",
            session_id, checkpoint_id, config.model, output_language
        ),
    );
    match state
        .qa_ai_use_case
        .generate_checkpoint_summary(&session_id, &checkpoint_id, &config, &output_language)
        .await
    {
        Ok(summary) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA summary generated (checkpoint_id={} summary_id={})",
                    checkpoint_id, summary.id
                ),
            );
            Ok(summary)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to generate summary (checkpoint_id={}): {}",
                    checkpoint_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_generate_test_cases(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    checkpoint_id: String,
    config: LLMConfig,
    output_language: String,
) -> Result<Vec<QaTestCase>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA generate test cases requested (session_id={} checkpoint_id={} model={} language={})",
            session_id, checkpoint_id, config.model, output_language
        ),
    );
    match state
        .qa_ai_use_case
        .generate_test_cases(&session_id, &checkpoint_id, &config, &output_language)
        .await
    {
        Ok(cases) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA test cases generated (checkpoint_id={} count={})",
                    checkpoint_id,
                    cases.len()
                ),
            );
            emit_status_log(
                &app,
                &state.logs,
                "INFO",
                "QA",
                "QA test cases generated",
                "success",
                None,
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: Some("ai_generate".to_string()),
                    mode: Some("api".to_string()),
                    event_type: Some("ai_generate_test_cases".to_string()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            Ok(cases)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to generate test cases (checkpoint_id={}): {}",
                    checkpoint_id, err
                ),
            );
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to generate test cases",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: Some("ai_generate".to_string()),
                    mode: Some("api".to_string()),
                    event_type: Some("ai_generate_test_cases".to_string()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_list_checkpoint_summaries(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<QaCheckpointSummary>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA list checkpoint summaries requested (session_id={})",
            session_id
        ),
    );
    match state
        .qa_ai_use_case
        .list_checkpoint_summaries(&session_id)
        .await
    {
        Ok(summaries) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA list checkpoint summaries success (session_id={} count={})",
                    session_id,
                    summaries.len()
                ),
            );
            Ok(summaries)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list checkpoint summaries (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_list_test_cases(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<QaTestCase>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list test cases requested (session_id={})", session_id),
    );
    match state.qa_ai_use_case.list_test_cases(&session_id).await {
        Ok(cases) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA list test cases success (session_id={} count={})",
                    session_id,
                    cases.len()
                ),
            );
            Ok(cases)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list test cases (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn qa_list_llm_runs(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<Vec<QaLlmRun>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list LLM runs requested (session_id={})", session_id),
    );
    match state.qa_ai_use_case.list_llm_runs(&session_id).await {
        Ok(runs) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA list LLM runs success (session_id={} count={})",
                    session_id,
                    runs.len()
                ),
            );
            Ok(runs)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to list LLM runs (session_id={}): {}",
                    session_id, err
                ),
            );
            Err(err)
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum QaCaptureMode {
    FullScreen,
    WindowedFrame,
}

#[derive(Debug, Clone, Copy)]
struct QaCaptureRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl QaCaptureRect {
    fn right(self) -> i32 {
        self.x + self.width as i32
    }

    fn bottom(self) -> i32 {
        self.y + self.height as i32
    }
}

fn clamp_rect(rect: QaCaptureRect, bounds: QaCaptureRect) -> QaCaptureRect {
    let left = rect.x.max(bounds.x);
    let top = rect.y.max(bounds.y);
    let right = rect.right().min(bounds.right());
    let bottom = rect.bottom().min(bounds.bottom());
    let width = (right - left).max(0) as u32;
    let height = (bottom - top).max(0) as u32;
    QaCaptureRect {
        x: left,
        y: top,
        width,
        height,
    }
}

fn intersection_area(rect: QaCaptureRect, bounds: QaCaptureRect) -> u64 {
    let clamped = clamp_rect(rect, bounds);
    u64::from(clamped.width) * u64::from(clamped.height)
}

/// Capture a native screenshot of a screen region.
/// Coordinates are in screen coordinates (not window-relative).
#[tauri::command]
pub async fn qa_capture_native_screenshot(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    event_id: Option<String>,
    capture_mode: Option<QaCaptureMode>,
) -> Result<QaScreenshotResult> {
    use screenshots::Screen;
    use std::io::Cursor;

    let session_id = session_id.trim().to_string();

    if session_id.is_empty() {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "Native screenshot: session id missing",
        );
        return Err(AppError::ValidationError(
            "Session id is required.".to_string(),
        ));
    }

    let requested_mode = capture_mode.unwrap_or(QaCaptureMode::WindowedFrame);

    // Get all screens and find the one containing the region
    let screens = Screen::all().map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to get screens: {}", e),
        );
        AppError::Internal(format!("Failed to get screens: {}", e))
    })?;

    if screens.is_empty() {
        add_log(&state.logs, "ERROR", "QA", "No screens found");
        return Err(AppError::Internal("No screens found.".to_string()));
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_right = i32::MIN;
    let mut max_bottom = i32::MIN;
    for screen in &screens {
        let info = screen.display_info;
        min_x = min_x.min(info.x);
        min_y = min_y.min(info.y);
        max_right = max_right.max(info.x + info.width as i32);
        max_bottom = max_bottom.max(info.y + info.height as i32);
    }

    let workspace_bounds = QaCaptureRect {
        x: min_x,
        y: min_y,
        width: (max_right - min_x).max(0) as u32,
        height: (max_bottom - min_y).max(0) as u32,
    };

    let requested_rect = match requested_mode {
        QaCaptureMode::FullScreen => workspace_bounds,
        QaCaptureMode::WindowedFrame => QaCaptureRect {
            x,
            y,
            width,
            height,
        },
    };

    let mode_label = match requested_mode {
        QaCaptureMode::FullScreen => "full_screen",
        QaCaptureMode::WindowedFrame => "windowed_frame",
    };
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA native screenshot requested (session_id={} mode={} region={}x{}+{}+{} event_id={})",
            session_id,
            mode_label,
            requested_rect.width,
            requested_rect.height,
            requested_rect.x,
            requested_rect.y,
            event_id.as_deref().unwrap_or("-")
        ),
    );

    if requested_rect.width == 0 || requested_rect.height == 0 {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "Native screenshot: invalid dimensions",
        );
        return Err(AppError::ValidationError(
            "Invalid screenshot dimensions.".to_string(),
        ));
    }

    let clamped_workspace = clamp_rect(requested_rect, workspace_bounds);
    if clamped_workspace.width == 0 || clamped_workspace.height == 0 {
        add_log(
            &state.logs,
            "INFO",
            "QA",
            "Native screenshot skipped: region outside workspace",
        );
        return Err(AppError::ValidationError(
            "Screenshot region is outside active workspace.".to_string(),
        ));
    }

    if clamped_workspace.x != requested_rect.x
        || clamped_workspace.y != requested_rect.y
        || clamped_workspace.width != requested_rect.width
        || clamped_workspace.height != requested_rect.height
    {
        add_log(
            &state.logs,
            "INFO",
            "QA",
            &format!(
                "Native screenshot clamped to workspace: {}x{}+{}+{}",
                clamped_workspace.width,
                clamped_workspace.height,
                clamped_workspace.x,
                clamped_workspace.y
            ),
        );
    }

    let mut target_screen = &screens[0];
    let mut target_bounds = QaCaptureRect {
        x: screens[0].display_info.x,
        y: screens[0].display_info.y,
        width: screens[0].display_info.width,
        height: screens[0].display_info.height,
    };
    let mut best_area = 0u64;

    for screen in &screens {
        let info = screen.display_info;
        let bounds = QaCaptureRect {
            x: info.x,
            y: info.y,
            width: info.width,
            height: info.height,
        };
        let area = intersection_area(clamped_workspace, bounds);
        if area > best_area {
            best_area = area;
            target_screen = screen;
            target_bounds = bounds;
        }
    }

    let clamped_target = clamp_rect(clamped_workspace, target_bounds);
    if clamped_target.width == 0 || clamped_target.height == 0 {
        add_log(
            &state.logs,
            "INFO",
            "QA",
            "Native screenshot skipped: region outside target screen",
        );
        return Err(AppError::ValidationError(
            "Screenshot region is outside target screen.".to_string(),
        ));
    }

    if clamped_target.x != clamped_workspace.x
        || clamped_target.y != clamped_workspace.y
        || clamped_target.width != clamped_workspace.width
        || clamped_target.height != clamped_workspace.height
    {
        add_log(
            &state.logs,
            "INFO",
            "QA",
            &format!(
                "Native screenshot clamped to screen: {}x{}+{}+{}",
                clamped_target.width, clamped_target.height, clamped_target.x, clamped_target.y
            ),
        );
    }

    // Capture the region
    let image = target_screen
        .capture_area(
            clamped_target.x,
            clamped_target.y,
            clamped_target.width,
            clamped_target.height,
        )
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to capture region: {}", e),
            );
            AppError::Internal(format!("Failed to capture screen region: {}", e))
        })?;

    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    {
        use screenshots::image::codecs::png::PngEncoder;
        use screenshots::image::ImageEncoder;
        let encoder = PngEncoder::new(Cursor::new(&mut png_bytes));
        encoder
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                screenshots::image::ColorType::Rgba8,
            )
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "QA",
                    &format!("Failed to encode PNG: {}", e),
                );
                AppError::Internal(format!("Failed to encode screenshot: {}", e))
            })?;
    }

    // Save to file
    let app_data_dir = resolve_app_data_dir(&app).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve app data dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;
    let qa_sessions_dir = ensure_qa_sessions_root(&app_data_dir).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve QA sessions dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;
    let session_dir = ensure_session_dir(&qa_sessions_dir, &session_id).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve session dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;
    let screenshots_dir = ensure_session_screenshots_dir(&session_dir).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to resolve screenshots dir: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;

    let now = chrono::Utc::now().timestamp_millis();
    let artifact_id = Uuid::new_v4().to_string();
    let filename = format!("screenshot_{}_{}.png", now, &artifact_id[..8]);
    let path = screenshots_dir.join(&filename);

    std::fs::write(&path, &png_bytes).map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to save screenshot: {}", err),
        );
        AppError::Internal(err.to_string())
    })?;

    // Attach to event
    let resolved_event_id = match state
        .qa_event_use_case
        .attach_screenshot(
            &session_id,
            event_id,
            &artifact_id,
            path.to_string_lossy().as_ref(),
            Some("image/png"),
            None,
            None,
            now,
        )
        .await
    {
        Ok(event_id) => event_id,
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to attach native screenshot");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to attach native screenshot (session_id={}): {}",
                    session_id, err
                ),
            );
            return Err(err);
        }
    };

    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA native screenshot saved: session_id={} event_id={} path={} size={}",
            session_id,
            resolved_event_id,
            path.display(),
            png_bytes.len()
        ),
    );

    Ok(QaScreenshotResult {
        path: path.to_string_lossy().to_string(),
        event_id: resolved_event_id,
        artifact_id,
    })
}

#[tauri::command]
pub async fn qa_explore_session(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    config: LLMConfig,
    output_language: String,
) -> Result<ExploreResult> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA explore session requested (session_id={} model={} language={})",
            session_id, config.model, output_language
        ),
    );
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "Starting AI exploration",
        "running",
        None,
        Some(QaLogContext {
            session_id: Some(session_id.clone()),
            run_id: None,
            run_type: Some("ai_explore".to_string()),
            mode: Some("browser".to_string()),
            event_type: Some("explore_session".to_string()),
            status_code: None,
            latency_ms: None,
        }),
    );

    match state
        .qa_ai_use_case
        .explore_and_generate_tests(&session_id, &config, &output_language)
        .await
    {
        Ok(result) => {
            let checkpoint_count = result.checkpoints.len();
            let summary_count = result.summaries.len();
            let test_case_count = result.test_cases.len();
            let llm_run_count = result.llm_runs.len();
            let patterns = result.detected_patterns.join(", ");

            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA explore session completed: checkpoints={} summaries={} test_cases={} llm_runs={} patterns=[{}]",
                    checkpoint_count, summary_count, test_case_count, llm_run_count, patterns
                ),
            );
            emit_status_log(
                &app,
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "AI exploration complete: {} test cases generated",
                    test_case_count
                ),
                "success",
                None,
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: Some("ai_explore".to_string()),
                    mode: Some("browser".to_string()),
                    event_type: Some("explore_session".to_string()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            Ok(result)
        }
        Err(err) => {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "QA explore session failed (session_id={}): {}",
                    session_id, err
                ),
            );
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "AI exploration failed",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: Some("ai_explore".to_string()),
                    mode: Some("browser".to_string()),
                    event_type: Some("explore_session".to_string()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            Err(err)
        }
    }
}
