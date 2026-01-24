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

use super::logging::{emit_status_log, QaLogContext, QA_EVENT_EMIT, QA_RUN_STREAM_EMIT};

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
