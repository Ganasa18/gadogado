use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

use super::template_sql::hash_query;
use super::super::types::{TemplateFeedbackRequest, TemplateFeedbackResponse};

pub async fn submit_template_feedback_impl(
    state: State<'_, Arc<super::super::AppState>>,
    request: TemplateFeedbackRequest,
) -> Result<TemplateFeedbackResponse> {
    let query_hash = hash_query(&request.query);

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Recording template feedback: query_hash={}, auto={:?}, user={}",
            query_hash, request.auto_selected_template_id, request.user_selected_template_id
        ),
    );

    match state
        .rag_repository
        .record_template_feedback(
            &query_hash,
            request.collection_id,
            request.auto_selected_template_id,
            request.user_selected_template_id,
        )
        .await
    {
        Ok(_) => {
            add_log(
                &state.logs,
                "DEBUG",
                "SQL-RAG",
                "Template feedback recorded successfully",
            );
            Ok(TemplateFeedbackResponse {
                success: true,
                message: "Feedback recorded. Future similar queries will prioritize this template."
                    .to_string(),
            })
        }
        Err(e) => {
            add_log(
                &state.logs,
                "WARN",
                "SQL-RAG",
                &format!("Failed to record template feedback: {}", e),
            );
            Ok(TemplateFeedbackResponse {
                success: false,
                message: format!("Could not record feedback: {}", e),
            })
        }
    }
}
