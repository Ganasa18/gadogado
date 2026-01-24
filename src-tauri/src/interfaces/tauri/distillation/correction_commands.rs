//! Correction Commands (Flow A - Collect Corrections)
use crate::domain::error::Result;
use crate::infrastructure::db::training::repositories::{
    Correction, CorrectionInput, CorrectionRepository, Tag, TagRepository, TrainingDb,
};
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tauri::{AppHandle, State};

use super::common::training_db_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionWithTags {
    #[serde(flatten)]
    pub correction: Correction,
    pub tags: Vec<Tag>,
}

#[tauri::command]
pub async fn distill_save_correction(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: CorrectionInput,
    tags: Option<Vec<String>>,
) -> Result<Correction> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Saving correction: {} (prompt length: {})",
            input.correction_id,
            input.prompt.len()
        ),
    );

    let db_path = training_db_path(&app)?;
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!("DB path: {:?}", db_path),
    );

    let db = TrainingDb::connect(&db_path).await?;
    add_log(&state.logs, "DEBUG", "Distillation", "Connected to DB");

    // Debug: verify schema on the exact connection we will use.
    if let Ok(rows) = sqlx::query("PRAGMA table_info(corrections)")
        .fetch_all(db.pool())
        .await
    {
        let cols: Vec<String> = rows
            .into_iter()
            .filter_map(|r| r.try_get::<String, _>("name").ok())
            .collect();
        add_log(
            &state.logs,
            "DEBUG",
            "Distillation",
            &format!("corrections columns: {}", cols.join(", ")),
        );
    }

    let repo = CorrectionRepository::new(&db);

    repo.insert(&input).await?;
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        "Inserted correction into DB",
    );

    let correction = repo.get(&input.correction_id).await?;
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        "Retrieved correction from DB",
    );

    // Handle tags if provided
    if let Some(tag_names) = tags {
        add_log(
            &state.logs,
            "DEBUG",
            "Distillation",
            &format!("Adding {} tags to correction", tag_names.len()),
        );
        let tag_repo = TagRepository::new(&db);
        for tag_name in tag_names {
            let tag_id = tag_repo.get_or_create(&tag_name).await?;
            tag_repo
                .add_to_correction(&input.correction_id, tag_id)
                .await?;
        }
    }

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Correction saved successfully: {}", input.correction_id),
    );

    Ok(correction)
}

#[tauri::command]
pub async fn distill_get_correction(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    correction_id: String,
) -> Result<CorrectionWithTags> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;

    let correction_repo = CorrectionRepository::new(&db);
    let tag_repo = TagRepository::new(&db);

    let correction = correction_repo.get(&correction_id).await?;
    let tags = tag_repo.list_for_correction(&correction_id).await?;

    Ok(CorrectionWithTags { correction, tags })
}

#[tauri::command]
pub async fn distill_list_corrections(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    limit: Option<i64>,
) -> Result<Vec<Correction>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = CorrectionRepository::new(&db);

    let corrections = repo.list_recent(limit.unwrap_or(100)).await?;

    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!("Listed {} corrections", corrections.len()),
    );

    Ok(corrections)
}

#[tauri::command]
pub async fn distill_delete_correction(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    correction_id: String,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Deleting correction: {}", correction_id),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = CorrectionRepository::new(&db);

    let rows = repo.delete(&correction_id).await?;
    Ok(rows)
}

#[tauri::command]
pub async fn distill_update_correction_tags(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    correction_id: String,
    tags: Vec<String>,
) -> Result<Vec<Tag>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let tag_repo = TagRepository::new(&db);

    // Get current tags
    let current_tags = tag_repo.list_for_correction(&correction_id).await?;
    let current_names: std::collections::HashSet<_> =
        current_tags.iter().map(|t| t.name.clone()).collect();
    let new_names: std::collections::HashSet<_> = tags.iter().cloned().collect();

    // Remove tags that are no longer in the list
    for tag in &current_tags {
        if !new_names.contains(&tag.name) {
            tag_repo
                .remove_from_correction(&correction_id, tag.tag_id)
                .await?;
        }
    }

    // Add new tags
    for tag_name in &tags {
        if !current_names.contains(tag_name) {
            let tag_id = tag_repo.get_or_create(tag_name).await?;
            tag_repo.add_to_correction(&correction_id, tag_id).await?;
        }
    }

    // Return updated tags
    let updated_tags = tag_repo.list_for_correction(&correction_id).await?;
    Ok(updated_tags)
}

#[tauri::command]
pub async fn distill_list_tags(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
) -> Result<Vec<Tag>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = TagRepository::new(&db);

    let tags = repo.list_all().await?;
    Ok(tags)
}
