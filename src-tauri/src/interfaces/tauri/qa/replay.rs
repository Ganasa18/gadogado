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

use super::types::{QaBrowserReplayEvent, QaBrowserReplayPayload};

#[tauri::command]
pub async fn qa_replay_browser(
    app: tauri::AppHandle,
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

    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resource_dir = app.path().resource_dir().ok();

    let mut script_candidates: Vec<PathBuf> = vec![
        // Development: script in src-tauri/resources/scripts folder
        cargo_manifest_dir.join("resources").join("scripts").join("qa-browser-replay.mjs"),
    ];

    // Production: bundled resources
    if let Some(ref res_dir) = resource_dir {
        script_candidates.push(res_dir.join("scripts").join("qa-browser-replay.mjs"));
    }

    let script_path = script_candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| {
            let candidates_str = script_candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            AppError::NotFound(format!(
                "QA replay script not found. Searched: {}",
                candidates_str
            ))
        })?;

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
