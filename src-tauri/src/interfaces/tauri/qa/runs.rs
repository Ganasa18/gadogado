use crate::domain::error::Result;
use crate::domain::qa_run::{QaRunStreamEvent, QaRunStreamInput, QaSessionRun};
use std::sync::Arc;
use tauri::{Emitter, State};

use crate::interfaces::http::add_log;


use crate::interfaces::tauri::AppState;

use super::logging::{emit_status_log, QaLogContext, QA_RUN_STREAM_EMIT, QA_RUN_UPDATED_EMIT};

#[tauri::command]
pub async fn qa_start_run(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
    run_type: String,
    mode: String,
    triggered_by: String,
    source_run_id: Option<String>,
    checkpoint_id: Option<String>,
    meta_json: Option<String>,
) -> Result<QaSessionRun> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA start run requested (session_id={}, type={})",
            session_id, run_type
        ),
    );
    let run = match state
        .qa_run_use_case
        .start_run(
            &session_id,
            &run_type,
            &mode,
            &triggered_by,
            source_run_id,
            checkpoint_id,
            meta_json,
        )
        .await
    {
        Ok(run) => run,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to start QA run",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: Some(session_id.clone()),
                    run_id: None,
                    run_type: Some(run_type.clone()),
                    mode: Some(mode.clone()),
                    event_type: None,
                    status_code: None,
                    latency_ms: None,
                }),
            );
            return Err(err);
        }
    };
    if let Err(err) = app.emit(QA_RUN_UPDATED_EMIT, &run) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit run update: {}", err),
        );
    }
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(
            &run.id,
            QaRunStreamInput {
                channel: "system".to_string(),
                level: "info".to_string(),
                message: format!("Run started ({})", run.run_type),
                payload_json: None,
            },
        )
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
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA run started",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(run.session_id.clone()),
            run_id: Some(run.id.clone()),
            run_type: Some(run.run_type.clone()),
            mode: Some(run.mode.clone()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );
    Ok(run)
}

#[tauri::command]
pub async fn qa_end_run(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
    status: String,
) -> Result<QaSessionRun> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "QA end run requested (run_id={}, status={})",
            run_id, status
        ),
    );
    let run = match state.qa_run_use_case.end_run(&run_id, &status).await {
        Ok(run) => run,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "Failed to end QA run",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: None,
                    run_id: Some(run_id.clone()),
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
    if let Err(err) = app.emit(QA_RUN_UPDATED_EMIT, &run) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit run update: {}", err),
        );
    }
    if let Ok(stream_event) = state
        .qa_run_use_case
        .append_stream_event(
            &run.id,
            QaRunStreamInput {
                channel: "system".to_string(),
                level: "info".to_string(),
                message: format!("Run ended ({})", run.status),
                payload_json: None,
            },
        )
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
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA run ended",
        "success",
        None,
        Some(QaLogContext {
            session_id: Some(run.session_id.clone()),
            run_id: Some(run.id.clone()),
            run_type: Some(run.run_type.clone()),
            mode: Some(run.mode.clone()),
            event_type: None,
            status_code: None,
            latency_ms: None,
        }),
    );
    Ok(run)
}

#[tauri::command]
pub async fn qa_append_run_stream_event(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
    event: QaRunStreamInput,
) -> Result<QaRunStreamEvent> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA append run stream event (run_id={})", run_id),
    );
    let stored = state
        .qa_run_use_case
        .append_stream_event(&run_id, event)
        .await?;
    if let Err(err) = app.emit(QA_RUN_STREAM_EMIT, &stored) {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            &format!("Failed to emit run stream: {}", err),
        );
    }
    Ok(stored)
}

#[tauri::command]
pub async fn qa_list_run_stream_events(
    state: State<'_, Arc<AppState>>,
    run_id: String,
    limit: Option<i64>,
) -> Result<Vec<QaRunStreamEvent>> {
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!("QA list run stream events (run_id={})", run_id),
    );
    state
        .qa_run_use_case
        .list_stream_events(&run_id, limit.unwrap_or(50))
        .await
}
