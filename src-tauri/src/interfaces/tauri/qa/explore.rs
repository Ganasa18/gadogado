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
