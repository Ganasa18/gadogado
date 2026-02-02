use crate::application::use_cases::qa_ai::ExploreResult;
use crate::domain::error::Result;
use crate::domain::llm_config::LLMConfig;
use std::sync::Arc;
use tauri::State;

use crate::interfaces::http::add_log;


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
