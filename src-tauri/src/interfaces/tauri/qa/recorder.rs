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
use crate::interfaces::tauri::QaRecorderHandle;

use super::logging::{emit_status_log, QaLogContext, QA_EVENT_EMIT, QA_LOG_EMIT, QA_RUN_STREAM_EMIT, QA_RUN_UPDATED_EMIT};

use super::recorder_internal::{
    record_recorder_event, record_recorder_network, RecorderAuthPayload, RecorderEventPayload,
    RecorderMessage, RecorderNetworkPayload, RecorderStatusPayload,
};

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

    // Resolve script path - try multiple locations for dev and production
    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resource_dir = app.path().resource_dir().ok();

    let mut script_candidates: Vec<PathBuf> = vec![
        // Development: script in src-tauri/resources/scripts folder
        cargo_manifest_dir.join("resources").join("scripts").join("qa-browser-recorder.mjs"),
    ];

    // Production: bundled resources
    if let Some(ref res_dir) = resource_dir {
        script_candidates.push(res_dir.join("scripts").join("qa-browser-recorder.mjs"));
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
                "QA recorder script not found. Searched: {}",
                candidates_str
            ))
        })?;

    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("Using recorder script at: {}", script_path.display()),
    );

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
