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

use super::logging::{emit_status_log, QaLogContext};

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
