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

use super::types::{QaApiFormField, QaApiKeyValue, QaApiRequest, QaApiResponse};

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
