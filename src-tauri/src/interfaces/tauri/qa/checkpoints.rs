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
