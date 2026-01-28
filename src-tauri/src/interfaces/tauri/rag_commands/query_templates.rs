//! Query Templates Commands (Feature 31)
//!
//! CRUD operations for managing query templates used in few-shot learning

use crate::domain::rag_entities::{QueryTemplate, QueryTemplateInput};
use crate::domain::error::Result;
use std::sync::Arc;
use tauri::State;

/// List query templates for a profile (or all if profile_id is None)
#[tauri::command]
pub async fn db_list_query_templates(
    state: State<'_, Arc<super::AppState>>,
    profile_id: Option<i64>,
) -> Result<Vec<QueryTemplate>> {
    state
        .rag_repository
        .list_query_templates(profile_id)
        .await
}

/// Create a new query template
#[tauri::command]
pub async fn db_create_query_template(
    state: State<'_, Arc<super::AppState>>,
    input: QueryTemplateInput,
) -> Result<QueryTemplate> {
    state
        .rag_repository
        .create_query_template(&input)
        .await
}

/// Update an existing query template
#[tauri::command]
pub async fn db_update_query_template(
    state: State<'_, Arc<super::AppState>>,
    template_id: i64,
    input: QueryTemplateInput,
) -> Result<QueryTemplate> {
    state
        .rag_repository
        .update_query_template(template_id, &input)
        .await
}

/// Delete a query template
#[tauri::command]
pub async fn db_delete_query_template(
    state: State<'_, Arc<super::AppState>>,
    template_id: i64,
) -> Result<()> {
    state
        .rag_repository
        .delete_query_template(template_id)
        .await
        .map(|_| ())
}

/// Toggle template enabled status
#[tauri::command]
pub async fn db_toggle_query_template(
    state: State<'_, Arc<super::AppState>>,
    template_id: i64,
    is_enabled: bool,
) -> Result<QueryTemplate> {
    state
        .rag_repository
        .toggle_query_template(template_id, is_enabled)
        .await
}
