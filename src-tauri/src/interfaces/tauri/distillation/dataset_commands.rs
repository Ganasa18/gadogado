//! Dataset Commands (Flow B - Prepare Training Dataset)
use crate::domain::error::Result;
use crate::infrastructure::db::training::repositories::{
    Dataset, DatasetInput, DatasetItem, DatasetItemInput, DatasetRepository, TrainingDb,
};
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

use super::common::training_db_path;

#[tauri::command]
pub async fn distill_create_dataset(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: DatasetInput,
) -> Result<Dataset> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Creating dataset: {}", input.name),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    repo.insert(&input).await?;
    let dataset = repo.get(&input.dataset_id).await?;

    Ok(dataset)
}

#[tauri::command]
pub async fn distill_get_dataset(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    dataset_id: String,
) -> Result<Dataset> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    repo.get(&dataset_id).await
}

#[tauri::command]
pub async fn distill_list_datasets(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    dataset_type: Option<String>,
) -> Result<Vec<Dataset>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    match dataset_type {
        Some(dt) => repo.list_by_type(&dt).await,
        None => repo.list_all().await,
    }
}

#[tauri::command]
pub async fn distill_delete_dataset(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    dataset_id: String,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Deleting dataset: {}", dataset_id),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    repo.delete(&dataset_id).await
}

#[tauri::command]
pub async fn distill_add_dataset_item(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    item: DatasetItemInput,
) -> Result<()> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    repo.insert_item(&item).await
}

#[tauri::command]
pub async fn distill_list_dataset_items(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    dataset_id: String,
) -> Result<Vec<DatasetItem>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    repo.list_items(&dataset_id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetImportInput {
    pub dataset_id: Option<String>,
    pub name: String,
    pub dataset_type: String,
    pub description: Option<String>,
    pub path: String,
}

#[tauri::command]
pub async fn distill_import_dataset_jsonl(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: DatasetImportInput,
) -> Result<Dataset> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Importing dataset JSONL: {}", input.name),
    );

    let dataset_id = input
        .dataset_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let dataset_input = DatasetInput {
        dataset_id: dataset_id.clone(),
        name: input.name.clone(),
        dataset_type: input.dataset_type.clone(),
        description: input.description.clone(),
    };

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = DatasetRepository::new(&db);

    repo.insert(&dataset_input).await?;

    let file = std::fs::File::open(&input.path).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to open dataset file {}: {e}",
            input.path
        ))
    })?;
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line.map_err(|e| {
            crate::domain::error::AppError::Internal(format!("Failed to read dataset line: {e}"))
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            crate::domain::error::AppError::ValidationError(format!("Invalid JSONL row: {e}"))
        })?;

        let prompt = value
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if prompt.is_empty() {
            continue;
        }

        let expected_output = value
            .get("expected_output")
            .or_else(|| value.get("expectedOutput"))
            .or_else(|| value.get("target"))
            .or_else(|| value.get("output"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let metadata_json = value
            .get("metadata")
            .or_else(|| value.get("metadata_json"))
            .and_then(|v| serde_json::to_string(v).ok());

        repo.insert_item(&DatasetItemInput {
            item_id: Uuid::new_v4().to_string(),
            dataset_id: dataset_id.clone(),
            prompt,
            expected_output,
            metadata_json,
            source_correction_id: None,
        })
        .await?;
    }

    repo.get(&dataset_id).await
}
