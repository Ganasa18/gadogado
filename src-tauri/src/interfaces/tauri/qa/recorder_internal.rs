use crate::domain::error::Result;
use crate::domain::qa_event::QaEventInput;
use crate::domain::qa_run::QaRunStreamInput;
use std::sync::Arc;
use tauri::Emitter;

use crate::interfaces::http::add_log;

use serde::Deserialize;
use serde_json::{self, Value as JsonValue};

use crate::interfaces::tauri::AppState;

use super::logging::{QA_EVENT_EMIT, QA_RUN_STREAM_EMIT};
use super::screenshots::persist_screenshot_data_url;

pub(crate) async fn record_recorder_event(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    session_id: &str,
    run_id: &str,
    mode: &str,
    payload: RecorderEventPayload,
) -> Result<()> {
    let event_type = payload.event_type.to_lowercase();
    if !matches!(event_type.as_str(), "click" | "input" | "submit") {
        return Ok(());
    }

    let previous_event = state
        .qa_event_use_case
        .latest_event_summary(session_id)
        .await?;

    let event_input = QaEventInput {
        event_type: event_type.clone(),
        selector: payload.selector.clone(),
        element_text: payload.element_text.clone(),
        value: payload.value.clone(),
        url: payload.url.clone(),
        meta_json: payload.meta.map(|meta| meta.to_string()),
        run_id: Some(run_id.to_string()),
        checkpoint_id: None,
        origin: Some(
            payload
                .origin
                .clone()
                .unwrap_or_else(|| if mode == "ai" { "ai" } else { "user" }.to_string()),
        ),
        recording_mode: Some("browser".to_string()),
    };

    let recorded = state
        .qa_event_use_case
        .record_event(session_id, event_input)
        .await?;

    if let Some(data_url) = payload.screenshot_data_url.as_deref() {
        add_log(
            &state.logs,
            "INFO",
            "QA",
            &format!(
                "QA recorder screenshot received (session_id={} event_id={})",
                session_id, recorded.id
            ),
        );
        if let Err(err) =
            persist_screenshot_data_url(app, state, session_id, data_url, Some(&recorded.id)).await
        {
            add_log(
                &state.logs,
                "ERROR",
                "QA",
                &format!(
                    "Failed to save recorder screenshot (session_id={} event_id={}): {}",
                    session_id, recorded.id, err
                ),
            );
        }
    }

    if let Err(err) = app.emit(QA_EVENT_EMIT, &recorded) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit QA event: {}", err),
        );
    }

    let stream_input = QaRunStreamInput {
        channel: "browser".to_string(),
        level: "info".to_string(),
        message: format!("Event recorded: {}", recorded.event_type),
        payload_json: serde_json::to_string(&recorded).ok(),
    };
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(run_id, stream_input)
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

    if let Ok(checkpoints) = state
        .qa_ai_use_case
        .maybe_create_checkpoint_from_event(session_id, &recorded, previous_event)
        .await
    {
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

    Ok(())
}

pub(crate) async fn record_recorder_network(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    session_id: &str,
    run_id: &str,
    payload: RecorderNetworkPayload,
) -> Result<()> {
    let request_headers_json = payload
        .request_headers
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok());
    let response_headers_json = payload
        .response_headers
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok());
    let method = payload.method.clone();
    let url = payload.url.clone();

    let _call = state
        .qa_api_call_use_case
        .record_api_call(
            session_id,
            run_id,
            &method,
            &url,
            request_headers_json,
            payload.request_body.clone(),
            payload.status,
            response_headers_json,
            payload.response_body.clone(),
            payload.timing_ms,
        )
        .await?;

    let meta_json = serde_json::json!({
        "method": method,
        "url": url,
        "status": payload.status,
        "timing_ms": payload.timing_ms
    })
    .to_string();
    let event_input = QaEventInput {
        event_type: "api_response".to_string(),
        selector: None,
        element_text: None,
        value: None,
        url: Some(payload.url.clone()),
        meta_json: Some(meta_json),
        run_id: Some(run_id.to_string()),
        checkpoint_id: None,
        origin: Some("system".to_string()),
        recording_mode: Some("browser".to_string()),
    };
    let recorded = state
        .qa_event_use_case
        .record_event(session_id, event_input)
        .await?;

    let stream_input = QaRunStreamInput {
        channel: "api".to_string(),
        level: "info".to_string(),
        message: format!(
            "Network response: {} {}",
            method,
            payload.status.unwrap_or_default()
        ),
        payload_json: serde_json::to_string(&recorded).ok(),
    };
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(run_id, stream_input)
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

    Ok(())
}

#[derive(Debug, Deserialize)]
pub(crate) struct RecorderMessage {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) payload: JsonValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecorderEventPayload {
    pub(crate) event_type: String,
    pub(crate) selector: Option<String>,
    pub(crate) element_text: Option<String>,
    pub(crate) value: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) meta: Option<JsonValue>,
    pub(crate) origin: Option<String>,
    pub(crate) screenshot_data_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecorderNetworkPayload {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) status: Option<i64>,
    pub(crate) timing_ms: Option<i64>,
    pub(crate) request_headers: Option<JsonValue>,
    pub(crate) response_headers: Option<JsonValue>,
    pub(crate) request_body: Option<String>,
    pub(crate) response_body: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecorderStatusPayload {
    pub(crate) level: String,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecorderAuthPayload {
    pub(crate) path: String,
}


