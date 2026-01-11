use crate::application::use_cases::enhance::EnhanceUseCase;
use crate::application::use_cases::qa_event::QaEventUseCase;
use crate::application::use_cases::qa_session::QaSessionUseCase;
use crate::application::use_cases::translate::TranslateUseCase;
use crate::application::use_cases::typegen::TypeGenUseCase;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::prompt::Prompt;
use crate::domain::qa_event::{QaEvent, QaEventInput, QaEventPage};
use crate::domain::qa_session::QaSession;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::storage::{
    ensure_qa_sessions_root, ensure_session_dir, ensure_session_screenshots_dir,
    resolve_app_data_dir,
};
use base64::Engine;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::error;
use uuid::Uuid;

use crate::infrastructure::config::ConfigService;
use crate::infrastructure::llm_clients::LLMClient;
use crate::interfaces::http::add_log;

pub struct AppState {
    pub translate_use_case: TranslateUseCase,
    pub enhance_use_case: EnhanceUseCase,
    pub typegen_use_case: TypeGenUseCase,
    pub qa_session_use_case: QaSessionUseCase,
    pub qa_event_use_case: QaEventUseCase,
    pub qa_session_id: Mutex<Option<String>>,
    pub repository: Arc<SqliteRepository>,
    pub config_service: ConfigService,
    pub llm_client: Arc<dyn LLMClient + Send + Sync>,
    pub last_config: Mutex<LLMConfig>,
    pub preferred_source: Mutex<String>,
    pub preferred_target: Mutex<String>,
    pub logs: Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaScreenshotResult {
    pub path: String,
    pub event_id: String,
    pub artifact_id: String,
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
    *last_config = config;
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
pub async fn qa_start_session(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    title: String,
    goal: String,
    is_positive_case: bool,
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
        return Err(AppError::ValidationError(
            "A QA session is already active.".to_string(),
        ));
    }

    let app_version = Some(app.package_info().version.to_string());
    let os = Some(std::env::consts::OS.to_string());

    let session = match state
        .qa_session_use_case
        .start_session(title, goal, is_positive_case, app_version, os, notes)
        .await
    {
        Ok(session) => session,
        Err(err) => {
            error!(error = %err, "Failed to start QA session");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to start QA session: {}", err),
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
            "QA session started: id={} title=\"{}\" positive_case={}",
            session.id, session.title, session.is_positive_case
        ),
    );

    Ok(session)
}

#[tauri::command]
pub async fn qa_end_session(
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
        AppError::ValidationError("No active QA session to stop.".to_string())
    })?;

    let session = match state.qa_session_use_case.end_session(&session_id).await {
        Ok(session) => session,
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to end QA session");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to end QA session (id={}): {}", session_id, err),
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
        &format!(
            "QA list sessions requested (limit={})",
            limit.unwrap_or(50)
        ),
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
pub async fn qa_record_event(
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

    match state.qa_event_use_case.record_event(&session_id, event).await {
        Ok(recorded) => {
            add_log(
                &state.logs,
                "INFO",
                "QA",
                &format!(
                    "QA event recorded: type={} selector={} url={}",
                    recorded.event_type,
                    recorded
                        .selector
                        .as_deref()
                        .unwrap_or("-"),
                    recorded.url.as_deref().unwrap_or("-")
                ),
            );
            Ok(recorded)
        }
        Err(err) => {
            error!(error = %err, session_id = %session_id, "Failed to record QA event");
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!("Failed to record QA event (session_id={}): {}", session_id, err),
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
                &format!("Failed to list QA events (session_id={}): {}", session_id, err),
            );
            Err(err)
        }
    }
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

    let (mime, bytes) = match decode_data_url(&data_url) {
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
            &session_id,
            event_id,
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
                &format!("Failed to attach screenshot (session_id={}): {}", session_id, err),
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

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
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
