//! DB Query RAG Command
//!
//! This module is intentionally split into submodules to keep each file
//! small (<~500 LOC) and easier to maintain.

mod constants;
mod helpers;
mod results;
mod nl;
mod template_batching;
mod template_semantic;
mod template_llm;
mod template_sql;
mod flow_resolve;
mod flow_execute;

mod commands_query;
mod commands_template;
mod commands_feedback;

use crate::domain::error::Result;
use std::sync::Arc;
use tauri::State;

use super::types::{DbQueryRequest, DbQueryResponse, DbQueryWithTemplateRequest, TemplateFeedbackRequest, TemplateFeedbackResponse};

#[tauri::command]
pub async fn db_query_rag(
    state: State<'_, Arc<super::AppState>>,
    request: DbQueryRequest,
) -> Result<DbQueryResponse> {
    commands_query::db_query_rag_impl(state, request).await
}

#[tauri::command]
pub async fn db_query_rag_with_template(
    state: State<'_, Arc<super::AppState>>,
    request: DbQueryWithTemplateRequest,
) -> Result<DbQueryResponse> {
    commands_template::db_query_rag_with_template_impl(state, request).await
}

#[tauri::command]
pub async fn submit_template_feedback(
    state: State<'_, Arc<super::AppState>>,
    request: TemplateFeedbackRequest,
) -> Result<TemplateFeedbackResponse> {
    commands_feedback::submit_template_feedback_impl(state, request).await
}
