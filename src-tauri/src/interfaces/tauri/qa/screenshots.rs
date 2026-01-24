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

use super::types::QaScreenshotResult;

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

pub(crate) async fn persist_screenshot_data_url(
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
