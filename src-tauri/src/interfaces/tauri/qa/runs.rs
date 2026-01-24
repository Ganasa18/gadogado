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

use crate::interfaces::tauri::AppState;

use super::logging::{emit_status_log, QaLogContext, QA_RUN_STREAM_EMIT, QA_RUN_UPDATED_EMIT};

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
