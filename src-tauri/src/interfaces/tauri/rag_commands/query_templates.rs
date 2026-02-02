//! Query Templates Commands (Feature 31)
//!
//! CRUD operations for managing query templates used in few-shot learning

use crate::domain::rag_entities::{
    QueryTemplate, QueryTemplateImportPreview, QueryTemplateImportResult, QueryTemplateInput,
};
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

/// Import query templates from a SQL file (e.g. backup_templates.sql)
///
/// The import is restricted to INSERT statements targeting db_query_templates.
#[tauri::command]
pub async fn db_import_query_templates_from_sql_file(
    state: State<'_, Arc<super::AppState>>,
    file_path: String,
) -> Result<i64> {
    state
        .rag_repository
        .import_query_templates_from_sql_file(&file_path)
        .await
}

/// Preview query template import from a SQL file.
///
/// This parses INSERT statements targeting db_query_templates and returns a review payload
/// (validation issues + duplicate detection) before committing any changes.
#[tauri::command]
pub async fn db_preview_query_templates_import_from_sql_file(
    state: State<'_, Arc<super::AppState>>,
    file_path: String,
    target_profile_id: i64,
) -> Result<QueryTemplateImportPreview> {
    tracing::info!(
        "db_preview_query_templates_import_from_sql_file: target_profile_id={}, file_path={}",
        target_profile_id,
        file_path
    );
    state
        .rag_repository
        .preview_import_query_templates_from_sql_file(&file_path, target_profile_id)
        .await
}

/// Import selected query templates (from preview).
#[tauri::command]
pub async fn db_import_query_templates_from_preview(
    state: State<'_, Arc<super::AppState>>,
    target_profile_id: i64,
    items: Vec<QueryTemplateInput>,
) -> Result<QueryTemplateImportResult> {
    tracing::info!(
        "db_import_query_templates_from_preview: target_profile_id={}, items={}",
        target_profile_id,
        items.len()
    );
    state
        .rag_repository
        .import_query_templates_from_preview(target_profile_id, items)
        .await
}
