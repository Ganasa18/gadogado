use std::sync::{Arc, Mutex};
use tauri::Emitter;

use crate::interfaces::http::{add_log, add_log_entry, LogEntry};

use serde::Serialize;

pub(crate) const QA_EVENT_EMIT: &str = "qa-event-recorded";
pub(crate) const QA_RUN_STREAM_EMIT: &str = "qa-run-stream";
pub(crate) const QA_RUN_UPDATED_EMIT: &str = "qa-run-updated";
pub(crate) const QA_LOG_EMIT: &str = "qa-log";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QaLogContext {
    pub(crate) session_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) run_type: Option<String>,
    pub(crate) mode: Option<String>,
    pub(crate) event_type: Option<String>,
    pub(crate) status_code: Option<i64>,
    pub(crate) latency_ms: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QaLogEvent {
    time: String,
    level: String,
    source: String,
    message: String,
    status: String,
    error: Option<String>,
    context: Option<QaLogContext>,
}

pub(crate) fn emit_status_log(
    app: &tauri::AppHandle,
    logs: &Arc<Mutex<Vec<LogEntry>>>,
    level: &str,
    source: &str,
    message: &str,
    status: &str,
    error: Option<&str>,
    context: Option<QaLogContext>,
) {
    let entry = add_log_entry(logs, level, source, message);
    let payload = QaLogEvent {
        time: entry.time,
        level: entry.level,
        source: entry.source,
        message: entry.message,
        status: status.to_string(),
        error: error.map(|value| value.to_string()),
        context,
    };
    if let Err(err) = app.emit(QA_LOG_EMIT, payload) {
        add_log(
            logs,
            "ERROR",
            "QA",
            &format!("Failed to emit QA log event: {}", err),
        );
    }
}
