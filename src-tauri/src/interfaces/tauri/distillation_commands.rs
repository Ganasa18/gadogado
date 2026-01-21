use crate::domain::error::Result;
use crate::infrastructure::artifact_store::{
    backup_training_db, cleanup_old_backups, list_backups, restore_from_backup, BackupConfig,
    BackupInfo, TrainingArtifactLayout,
};
use crate::infrastructure::db::training::repositories::{
    ActiveModelRepository, Correction, CorrectionInput, CorrectionRepository, Dataset,
    DatasetInput, DatasetItem, DatasetItemInput, DatasetRepository, EvaluationMetric,
    EvaluationMetricInput, EvaluationMetricsRepository, Model, ModelInput, ModelRepository,
    ModelVersion, ModelVersionInput, ModelVersionRepository, RunArtifact, RunArtifactInput,
    RunArtifactsRepository, RunCorrectionsRepository, RunDatasetsRepository, SoftLabel,
    SoftLabelGenerationInput, SoftLabelGenerationResult, SoftLabelInput, SoftLabelRepository, Tag,
    TagRepository, TrainingDb, TrainingLog, TrainingLogInput, TrainingLogRepository, TrainingRun,
    TrainingRunInput, TrainingRunRepository, TrainingStatus,
};
use crate::infrastructure::storage::resolve_app_data_dir;
use crate::interfaces::http::add_log;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use serde_json::Value as JsonValue;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager, State};
use tokio::fs::OpenOptions as TokioOpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{sleep, Duration, Instant};
use tracing::error;
use uuid::Uuid;

use super::{AppState, DistillTrainerHandle};

fn training_db_path(app: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = resolve_app_data_dir(app)?;
    Ok(app_data_dir.join("training.db"))
}

// ============================================================================
// Correction Commands (Flow A - Collect Corrections)
// ============================================================================

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
    // This helps diagnose cases where the DB path looks correct but schema differs.
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

// ============================================================================
// Dataset Commands (Flow B - Prepare Training Dataset)
// ============================================================================

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

// ============================================================================
// Model Commands
// ============================================================================

#[tauri::command]
pub async fn distill_register_model(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: ModelInput,
) -> Result<Model> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Registering model: {}", input.display_name),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelRepository::new(&db);

    repo.insert(&input).await?;
    repo.get(&input.model_id).await
}

#[tauri::command]
pub async fn distill_list_models(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    provider: Option<String>,
) -> Result<Vec<Model>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelRepository::new(&db);

    match provider {
        Some(p) => repo.list_by_provider(&p).await,
        None => repo.list_all().await,
    }
}

#[tauri::command]
pub async fn distill_get_model(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    model_id: String,
) -> Result<Model> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelRepository::new(&db);

    repo.get(&model_id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseModelEntry {
    pub name: String,
    pub path: String,
    pub source: String,
    pub kind: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseModelImportInput {
    pub source_path: String,
    pub display_name: Option<String>,
    pub model_id: Option<String>,
    pub model_family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseModelImportResult {
    pub model: Model,
    pub entry: BaseModelEntry,
}

#[tauri::command]
pub async fn distill_list_base_models(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<BaseModelEntry>> {
    add_log(&state.logs, "INFO", "Distillation", "Listing base models");

    let mut entries: Vec<BaseModelEntry> = Vec::new();

    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);
    layout.ensure()?;

    // Try to resolve the resource directory
    let resource_result = app.path().resolve("models/base", BaseDirectory::Resource);
    add_log(&state.logs, "DEBUG", "Distillation", &format!("Resource resolve result: {:?}", resource_result));

    if let Ok(resource_base) = resource_result {
        add_log(&state.logs, "DEBUG", "Distillation", &format!("Resource base path: {}", resource_base.display()));
        add_log(&state.logs, "DEBUG", "Distillation", &format!("Resource base exists: {}", resource_base.exists()));

        if resource_base.exists() {
            match scan_base_models(&resource_base, "resource") {
                Ok(models) => {
                    for model in &models {
                        add_log(&state.logs, "DEBUG", "Distillation", &format!(
                            "Found model in resources: {} (path: {}, kind: {}, format: {})",
                            model.name, model.path, model.kind, model.format
                        ));
                    }
                    add_log(&state.logs, "INFO", "Distillation", &format!("Found {} models in resources", models.len()));
                    entries.extend(models);
                }
                Err(e) => {
                    add_log(&state.logs, "WARN", "Distillation", &format!("Failed to scan resource models: {}", e));
                }
            }
        } else {
            // In development mode, try to find resources relative to the executable
            #[cfg(debug_assertions)]
            {
                if let Ok(exe_path) = std::env::current_exe() {
                    // The exe is in target/debug/, so resources are in target/debug/resources/
                    if let Some(exe_dir) = exe_path.parent() {
                        let dev_resource_path = exe_dir.join("resources/models/base");
                        add_log(&state.logs, "INFO", "Distillation", &format!("Trying dev resource path: {}", dev_resource_path.display()));
                        if dev_resource_path.exists() {
                            match scan_base_models(&dev_resource_path, "resource") {
                                Ok(models) => {
                                    for model in &models {
                                        add_log(&state.logs, "DEBUG", "Distillation", &format!(
                                            "Found model in dev resources: {} (path: {}, kind: {}, format: {})",
                                            model.name, model.path, model.kind, model.format
                                        ));
                                    }
                                    add_log(&state.logs, "INFO", "Distillation", &format!("Found {} models in dev resources", models.len()));
                                    entries.extend(models);
                                }
                                Err(e) => {
                                    add_log(&state.logs, "WARN", "Distillation", &format!("Failed to scan dev resource models: {}", e));
                                }
                            }
                        } else {
                            add_log(&state.logs, "WARN", "Distillation", &format!("Dev resource path does not exist: {}", dev_resource_path.display()));
                        }
                    }
                }
            }
        }
    }

    if layout.models_base_dir().exists() {
        match scan_base_models(layout.models_base_dir(), "app_data") {
            Ok(models) => {
                for model in &models {
                    add_log(&state.logs, "DEBUG", "Distillation", &format!(
                        "Found model in app_data: {} (path: {}, kind: {}, format: {})",
                        model.name, model.path, model.kind, model.format
                    ));
                }
                add_log(&state.logs, "INFO", "Distillation", &format!("Found {} models in app_data", models.len()));
                entries.extend(models);
            }
            Err(e) => {
                add_log(&state.logs, "WARN", "Distillation", &format!("Failed to scan app_data models: {}", e));
            }
        }
    }

    add_log(&state.logs, "INFO", "Distillation", &format!("Total base models found: {}", entries.len()));
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

#[tauri::command]
pub async fn distill_import_base_model(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: BaseModelImportInput,
) -> Result<BaseModelImportResult> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Importing base model from {}", input.source_path),
    );

    let src = PathBuf::from(&input.source_path);
    let meta = std::fs::metadata(&src).map_err(|e| {
        crate::domain::error::AppError::NotFound(format!(
            "Base model path not found: {} ({e})",
            src.display()
        ))
    })?;

    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);
    layout.ensure()?;

    let display_name = input.display_name.clone().unwrap_or_else(|| {
        src.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("base-model")
            .to_string()
    });
    let sanitized = sanitize_model_name(&display_name);

    let target = if meta.is_dir() {
        unique_path(layout.models_base_dir(), &sanitized, None)?
    } else {
        let ext = src.extension().and_then(|s| s.to_str());
        unique_path(layout.models_base_dir(), &sanitized, ext)?
    };

    if meta.is_dir() {
        copy_dir_all(&src, &target)?;
    } else {
        std::fs::copy(&src, &target).map_err(|e| {
            crate::domain::error::AppError::Internal(format!(
                "Failed to copy base model {} to {}: {e}",
                src.display(),
                target.display()
            ))
        })?;
    }

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelRepository::new(&db);

    let existing = repo.list_all().await?;
    if let Some(found) = existing
        .iter()
        .find(|m| m.default_artifact_path.as_deref() == Some(target.to_string_lossy().as_ref()))
    {
        return Ok(BaseModelImportResult {
            model: found.clone(),
            entry: BaseModelEntry {
                name: display_name,
                path: target.to_string_lossy().to_string(),
                source: "app_data".to_string(),
                kind: if target.is_dir() { "dir" } else { "file" }.to_string(),
                format: detect_model_format(&target),
            },
        });
    }

    let mut model_id = input.model_id.clone().unwrap_or_else(|| sanitized.clone());
    if existing.iter().any(|m| m.model_id == model_id) {
        model_id = format!("{}-{}", model_id, &Uuid::new_v4().to_string()[..8]);
    }

    let model_input = ModelInput {
        model_id: model_id.clone(),
        display_name: display_name.clone(),
        provider: "local".to_string(),
        model_family: input.model_family.clone(),
        default_artifact_path: Some(target.to_string_lossy().to_string()),
    };
    repo.insert(&model_input).await?;
    let model = repo.get(&model_id).await?;

    Ok(BaseModelImportResult {
        model,
        entry: BaseModelEntry {
            name: display_name,
            path: target.to_string_lossy().to_string(),
            source: "app_data".to_string(),
            kind: if target.is_dir() { "dir" } else { "file" }.to_string(),
            format: detect_model_format(&target),
        },
    })
}

#[tauri::command]
pub async fn distill_download_default_model(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<BaseModelImportResult> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        "Downloading default TinyLlama model",
    );

    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);
    layout.ensure()?;

    let model_name = "tinyllama";
    let sanitized = sanitize_model_name(model_name);

    // Check if model already exists
    let existing_path = layout.models_base_dir().join(&sanitized);
    if existing_path.exists() {
        add_log(
            &state.logs,
            "INFO",
            "Distillation",
            "Default model already exists, using existing",
        );

        let db_path = training_db_path(&app)?;
        let db = TrainingDb::connect(&db_path).await?;
        let repo = ModelRepository::new(&db);

        let existing = repo.list_all().await?;
        if let Some(found) = existing.iter().find(|m| {
            m.default_artifact_path.as_deref() == Some(existing_path.to_string_lossy().as_ref())
        }) {
            return Ok(BaseModelImportResult {
                model: found.clone(),
                entry: BaseModelEntry {
                    name: model_name.to_string(),
                    path: existing_path.to_string_lossy().to_string(),
                    source: "app_data".to_string(),
                    kind: "dir".to_string(),
                    format: "hf".to_string(),
                },
            });
        }
    }

    // Create a placeholder directory (actual download would be implemented here)
    // For now, we'll create a minimal HuggingFace model structure
    std::fs::create_dir_all(&existing_path).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to create model directory {}: {e}",
            existing_path.display()
        ))
    })?;

    // Create a minimal config.json for the model
    let config_path = existing_path.join("config.json");
    let config_content = r#"{
        "architectures": ["LlamaForCausalLM"],
        "attention_bias": false,
        "attention_dropout": 0.0,
        "bos_token_id": 1,
        "eos_token_id": 2,
        "hidden_act": "silu",
        "hidden_size": 2048,
        "initializer_range": 0.02,
        "intermediate_size": 5632,
        "max_position_embeddings": 2048,
        "model_type": "llama",
        "num_attention_heads": 32,
        "num_hidden_layers": 22,
        "num_key_value_heads": 4,
        "pretraining_tp": 1,
        "rms_norm_eps": 1e-05,
        "rope_scaling": null,
        "rope_theta": 10000.0,
        "torch_dtype": "float32",
        "transformers_version": "4.35.0",
        "use_cache": true,
        "vocab_size": 32000
    }"#;

    std::fs::write(&config_path, config_content).map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to write config.json: {e}"))
    })?;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Created default model placeholder at {}",
            existing_path.display()
        ),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelRepository::new(&db);

    let model_input = ModelInput {
        model_id: sanitized.clone(),
        display_name: "TinyLlama (Default)".to_string(),
        provider: "local".to_string(),
        model_family: Some("Llama".to_string()),
        default_artifact_path: Some(existing_path.to_string_lossy().to_string()),
    };
    repo.insert(&model_input).await?;
    let model = repo.get(&sanitized).await?;

    Ok(BaseModelImportResult {
        model,
        entry: BaseModelEntry {
            name: model_name.to_string(),
            path: existing_path.to_string_lossy().to_string(),
            source: "app_data".to_string(),
            kind: "dir".to_string(),
            format: "hf".to_string(),
        },
    })
}

fn scan_base_models(dir: &Path, source: &str) -> Result<Vec<BaseModelEntry>> {
    let mut out = Vec::new();

    // Helper function to collect model files (recursively)
    fn collect_models(dir: &Path, source: &str, out: &mut Vec<BaseModelEntry>) -> Result<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            crate::domain::error::AppError::Internal(format!(
                "Failed to read base model dir {}: {e}",
                dir.display()
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                crate::domain::error::AppError::Internal(format!(
                    "Failed to read base model entry: {e}"
                ))
            })?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .starts_with('.')
            {
                continue;
            }

            if path.is_dir() {
                // Check if this directory is a valid HuggingFace model (has config.json)
                let config_path = path.join("config.json");
                if config_path.exists() {
                    // This is an HF model directory - create an entry for it
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            let uuid_str = Uuid::new_v4().to_string();
                            format!("model-{}", &uuid_str[..8])
                        });

                    out.push(BaseModelEntry {
                        name,
                        path: path.to_string_lossy().to_string(),
                        source: source.to_string(),
                        kind: "dir".to_string(),
                        format: "hf".to_string(),
                    });
                    // Don't recurse into HF model directories - we've already captured the model
                } else {
                    // Not an HF model directory, recurse to look for GGUF files inside
                    collect_models(&path, source, out)?;
                }
            } else {
                // Only process files (not directories) at this level
                let kind = "file";

                // Extract model name from path
                // For files: remove common model file suffixes and extensions
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| {
                        let mut name = s.to_string();
                        // Remove quantization suffixes (both underscore and hyphen variants)
                        for suffix in &[
                            "_q5_k_m", "_q4_k_m", "_q8_0", "_q6_k", "_q5_k_s", "_q4_k_s",
                            "-q5_k_m", "-q4_k_m", "-q8_0", "-q6_k", "-q5_k_s", "-q4_k_s",
                        ] {
                            name = name.strip_suffix(suffix).unwrap_or(&name).to_string();
                        }
                        // Remove instruction type suffix
                        name = name.strip_suffix("-instruct").unwrap_or(&name).to_string();
                        // Remove format suffixes (case-insensitive)
                        name = name.strip_suffix("-GGUF").unwrap_or(&name).to_string();
                        name = name.strip_suffix("-gguf").unwrap_or(&name).to_string();
                        name = name.strip_suffix(".gguf").unwrap_or(&name).to_string();
                        name = name.strip_suffix(".GGuf").unwrap_or(&name).to_string();
                        name = name.strip_suffix(".GGUF").unwrap_or(&name).to_string();
                        // Remove trailing hyphen if present
                        name = name.strip_suffix('-').unwrap_or(&name).to_string();
                        name
                    })
                    .unwrap_or_default();

                // If name is empty, use a default
                let name = if name.is_empty() {
                    let uuid_str = Uuid::new_v4().to_string();
                    format!("model-{}", &uuid_str[..8])
                } else {
                    name
                };

                out.push(BaseModelEntry {
                    name,
                    path: path.to_string_lossy().to_string(),
                    source: source.to_string(),
                    kind: kind.to_string(),
                    format: detect_model_format(&path),
                });
            }
        }
        Ok(())
    }

    collect_models(dir, source, &mut out)?;
    Ok(out)
}

fn detect_model_format(path: &Path) -> String {
    if path.is_file() {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("gguf") {
                return "gguf".to_string();
            }
            if ext.eq_ignore_ascii_case("bin") {
                return "bin".to_string();
            }
            if ext.eq_ignore_ascii_case("safetensors") {
                return "safetensors".to_string();
            }
        }
        return "unknown".to_string();
    }

    let config = path.join("config.json");
    if config.exists() {
        return "hf".to_string();
    }
    "unknown".to_string()
}

fn sanitize_model_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() {
            out.push('-');
        }
    }
    if out.is_empty() {
        "base-model".to_string()
    } else {
        out
    }
}

fn unique_path(base: &Path, name: &str, ext: Option<&str>) -> Result<PathBuf> {
    let mut candidate = match ext {
        Some(ext) => base.join(format!("{}.{}", name, ext)),
        None => base.join(name),
    };
    if !candidate.exists() {
        return Ok(candidate);
    }

    let short = Uuid::new_v4().to_string();
    let short = &short[..8];
    candidate = match ext {
        Some(ext) => base.join(format!("{}-{}.{}", name, short, ext)),
        None => base.join(format!("{}-{}", name, short)),
    };
    Ok(candidate)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to create dir {}: {e}",
            dst.display()
        ))
    })?;
    for entry in std::fs::read_dir(src).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to read dir {}: {e}",
            src.display()
        ))
    })? {
        let entry = entry.map_err(|e| {
            crate::domain::error::AppError::Internal(format!("Failed to read dir entry: {e}"))
        })?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(|e| {
                crate::domain::error::AppError::Internal(format!(
                    "Failed to copy {} to {}: {e}",
                    path.display(),
                    target.display()
                ))
            })?;
        }
    }
    Ok(())
}

// ============================================================================
// Training Run Commands (Flow C - Run Training)
// ============================================================================

#[tauri::command]
pub async fn distill_create_training_run(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: TrainingRunInput,
    correction_ids: Vec<(String, String, f64)>, // (correction_id, split, weight)
    dataset_ids: Option<Vec<(String, String, f64)>>, // (dataset_id, split, weight)
) -> Result<TrainingRun> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Creating training run: {} with {} corrections, student_model: {}",
            input.run_id,
            correction_ids.len(),
            input.student_model_id
        ),
    );

    let db_path = match training_db_path(&app) {
        Ok(path) => {
            add_log(
                &state.logs,
                "DEBUG",
                "Distillation",
                &format!("Database path: {:?}", path),
            );
            path
        }
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "Distillation",
                &format!("Failed to get training db path: {}", e),
            );
            return Err(e);
        }
    };

    let db = match TrainingDb::connect(&db_path).await {
        Ok(db) => {
            add_log(
                &state.logs,
                "DEBUG",
                "Distillation",
                "Connected to training database",
            );
            db
        }
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "Distillation",
                &format!("Failed to connect to database: {}", e),
            );
            return Err(e);
        }
    };

    let run_repo = TrainingRunRepository::new(&db);
    let corrections_repo = RunCorrectionsRepository::new(&db);
    let datasets_repo = RunDatasetsRepository::new(&db);

    // Create the run
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!("Inserting training run: {}", input.run_id),
    );
    match run_repo.insert(&input).await {
        Ok(_) => {
            add_log(
                &state.logs,
                "DEBUG",
                "Distillation",
                "Training run inserted successfully",
            );
        }
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "Distillation",
                &format!("Failed to insert training run: {}", e),
            );
            return Err(e);
        }
    }

    // Attach corrections
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!("Attaching {} corrections", correction_ids.len()),
    );
    for (correction_id, split, weight) in &correction_ids {
        add_log(
            &state.logs,
            "DEBUG",
            "Distillation",
            &format!(
                "Adding correction: {} with split={}, weight={}",
                correction_id, split, weight
            ),
        );
        match corrections_repo
            .add(&input.run_id, correction_id, split, *weight)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                add_log(
                    &state.logs,
                    "ERROR",
                    "Distillation",
                    &format!("Failed to add correction {}: {}", correction_id, e),
                );
                return Err(e);
            }
        }
    }

    // Attach datasets if provided
    if let Some(datasets) = &dataset_ids {
        add_log(
            &state.logs,
            "DEBUG",
            "Distillation",
            &format!("Attaching {} datasets", datasets.len()),
        );
        for (dataset_id, split, weight) in datasets {
            match datasets_repo
                .add(&input.run_id, dataset_id, split, *weight)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    add_log(
                        &state.logs,
                        "ERROR",
                        "Distillation",
                        &format!("Failed to add dataset {}: {}", dataset_id, e),
                    );
                    return Err(e);
                }
            }
        }
    }

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Training run {} created successfully", input.run_id),
    );

    run_repo.get(&input.run_id).await
}

#[tauri::command]
pub async fn distill_update_run_status(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
    status: String,
    failure_reason: Option<String>,
) -> Result<TrainingRun> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Updating run status: {} -> {}", run_id, status),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = TrainingRunRepository::new(&db);

    let training_status = match status.as_str() {
        "queued" => TrainingStatus::Queued,
        "running" => TrainingStatus::Running,
        "completed" => TrainingStatus::Completed,
        "failed" => TrainingStatus::Failed,
        "cancelled" => TrainingStatus::Cancelled,
        "rolled_back" => TrainingStatus::RolledBack,
        _ => TrainingStatus::Failed,
    };

    let end_time = if matches!(
        training_status,
        TrainingStatus::Completed | TrainingStatus::Failed | TrainingStatus::Cancelled
    ) {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        None
    };

    repo.set_status(&run_id, training_status, end_time, failure_reason)
        .await?;

    repo.get(&run_id).await
}

#[tauri::command]
pub async fn distill_get_training_run(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    run_id: String,
) -> Result<TrainingRun> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = TrainingRunRepository::new(&db);

    repo.get(&run_id).await
}

#[tauri::command]
pub async fn distill_list_training_runs(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    limit: Option<i64>,
) -> Result<Vec<TrainingRun>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = TrainingRunRepository::new(&db);

    repo.list_recent(limit.unwrap_or(50)).await
}

#[tauri::command]
pub async fn distill_log_training_step(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    log: TrainingLogInput,
) -> Result<()> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = TrainingLogRepository::new(&db);

    repo.insert(&log).await
}

#[tauri::command]
pub async fn distill_list_training_logs(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    run_id: String,
    limit: Option<i64>,
) -> Result<Vec<TrainingLog>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = TrainingLogRepository::new(&db);

    repo.list_for_run(&run_id, limit.unwrap_or(200)).await
}

// ============================================================================
// Model Version Commands (Flow E - Promote Version + Rollback)
// ============================================================================

#[tauri::command]
pub async fn distill_create_model_version(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: ModelVersionInput,
) -> Result<ModelVersion> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Creating model version: {}", input.version_id),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelVersionRepository::new(&db);

    repo.insert(&input).await?;
    repo.get(&input.version_id).await
}

#[tauri::command]
pub async fn distill_list_model_versions(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    model_id: Option<String>,
) -> Result<Vec<ModelVersion>> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        "Listing model versions",
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelVersionRepository::new(&db);

    match model_id {
        Some(model_id) => repo.list_by_model(&model_id).await,
        None => repo.list_all().await,
    }
}

#[tauri::command]
pub async fn distill_get_model_version(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    version_id: String,
) -> Result<ModelVersion> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = ModelVersionRepository::new(&db);

    repo.get(&version_id).await
}

/// Guardrail thresholds for promotion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PromotionGuardrails {
    /// Minimum exact_match score required (0.0 - 1.0)
    pub min_exact_match: Option<f64>,
    /// Minimum BLEU score required (0.0 - 1.0)
    pub min_bleu: Option<f64>,
    /// Minimum F1 score required (0.0 - 1.0)
    pub min_f1: Option<f64>,
    /// Whether to require at least one evaluation before promotion
    pub require_evaluation: Option<bool>,
    /// Skip guardrail checks (use with caution)
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionResult {
    pub success: bool,
    pub version_id: String,
    pub model_id: String,
    pub guardrail_checks: Vec<GuardrailCheck>,
    pub backup_created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailCheck {
    pub metric_name: String,
    pub required: f64,
    pub actual: Option<f64>,
    pub passed: bool,
}

#[tauri::command]
pub async fn distill_promote_version(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    model_id: String,
    version_id: String,
    guardrails: Option<PromotionGuardrails>,
) -> Result<PromotionResult> {
    use crate::domain::error::AppError;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Promoting version {} for model {}", version_id, model_id),
    );

    let db_path = training_db_path(&app)?;
    let app_data_dir = resolve_app_data_dir(&app)?;
    let db = TrainingDb::connect(&db_path).await?;

    let guardrails = guardrails.unwrap_or_default();
    let mut guardrail_checks = Vec::new();

    // Check guardrails unless force is set
    if !guardrails.force.unwrap_or(false) {
        let metrics_repo = EvaluationMetricsRepository::new(&db);
        let metrics = metrics_repo.list_for_version(&version_id).await?;

        // Check if evaluation is required
        if guardrails.require_evaluation.unwrap_or(false) && metrics.is_empty() {
            return Err(AppError::ValidationError(
                "No evaluation metrics found. Run evaluation before promotion.".to_string(),
            ));
        }

        // Helper to check a metric threshold
        let check_metric = |metric_name: &str, threshold: Option<f64>| -> GuardrailCheck {
            if let Some(required) = threshold {
                let actual = metrics
                    .iter()
                    .find(|m| m.metric_name == metric_name)
                    .map(|m| m.metric_value);

                let passed = actual.map(|v| v >= required).unwrap_or(false);

                GuardrailCheck {
                    metric_name: metric_name.to_string(),
                    required,
                    actual,
                    passed,
                }
            } else {
                GuardrailCheck {
                    metric_name: metric_name.to_string(),
                    required: 0.0,
                    actual: None,
                    passed: true, // No threshold set, so it passes
                }
            }
        };

        // Check each guardrail
        if guardrails.min_exact_match.is_some() {
            guardrail_checks.push(check_metric("exact_match", guardrails.min_exact_match));
        }
        if guardrails.min_bleu.is_some() {
            guardrail_checks.push(check_metric("bleu", guardrails.min_bleu));
        }
        if guardrails.min_f1.is_some() {
            guardrail_checks.push(check_metric("f1", guardrails.min_f1));
        }

        // Check if any guardrail failed
        let failed_checks: Vec<_> = guardrail_checks.iter().filter(|c| !c.passed).collect();
        if !failed_checks.is_empty() {
            let failures: Vec<String> = failed_checks
                .iter()
                .map(|c| {
                    format!(
                        "{}: required {:.2}, got {}",
                        c.metric_name,
                        c.required,
                        c.actual
                            .map(|v| format!("{:.2}", v))
                            .unwrap_or("N/A".to_string())
                    )
                })
                .collect();

            add_log(
                &state.logs,
                "WARN",
                "Distillation",
                &format!("Promotion blocked by guardrails: {}", failures.join(", ")),
            );

            return Err(AppError::ValidationError(format!(
                "Promotion blocked by guardrails: {}",
                failures.join("; ")
            )));
        }
    }

    // Create backup before promotion
    let backup_config = BackupConfig::new(&app_data_dir);
    let backup_created = backup_training_db(
        &db_path,
        &backup_config,
        Some(&format!(
            "pre_promote_{}",
            &version_id[..8.min(version_id.len())]
        )),
    )
    .is_ok();

    if !backup_created {
        error!("Failed to create backup before promotion");
    }

    let active_repo = ActiveModelRepository::new(&db);
    active_repo.set_active(&model_id, &version_id).await?;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Version {} is now active for model {}",
            version_id, model_id
        ),
    );

    Ok(PromotionResult {
        success: true,
        version_id,
        model_id,
        guardrail_checks,
        backup_created,
    })
}

#[tauri::command]
pub async fn distill_get_active_version(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    model_id: String,
) -> Result<Option<ModelVersion>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;

    let active_repo =
        crate::infrastructure::db::training::repositories::ActiveModelRepository::new(&db);
    let version_repo = ModelVersionRepository::new(&db);

    let version_id = active_repo.get_active_version_id(&model_id).await?;

    match version_id {
        Some(vid) => {
            let version = version_repo.get(&vid).await?;
            Ok(Some(version))
        }
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    pub previous_version_id: Option<String>,
    pub rolled_back_to: ModelVersion,
    pub backup_created: bool,
}

/// Rollback a model to a previous version.
/// This command:
/// 1. Creates a backup before rollback
/// 2. Verifies the target version exists and has valid artifacts
/// 3. Updates the active model pointer
/// 4. Returns rollback details
#[tauri::command]
pub async fn distill_rollback_version(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    model_id: String,
    target_version_id: String,
) -> Result<RollbackResult> {
    use crate::domain::error::AppError;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Rolling back model {} to version {}",
            model_id, target_version_id
        ),
    );

    let db_path = training_db_path(&app)?;
    let app_data_dir = resolve_app_data_dir(&app)?;

    // Create backup before rollback
    let backup_config = BackupConfig::new(&app_data_dir);
    let backup_created = backup_training_db(
        &db_path,
        &backup_config,
        Some(&format!(
            "pre_rollback_{}",
            &target_version_id[..8.min(target_version_id.len())]
        )),
    )
    .is_ok();

    let db = TrainingDb::connect(&db_path).await?;
    let version_repo = ModelVersionRepository::new(&db);
    let active_repo = ActiveModelRepository::new(&db);

    // Get current active version (if any)
    let previous_version_id = active_repo.get_active_version_id(&model_id).await?;

    // Verify target version exists
    let target_version = version_repo.get(&target_version_id).await?;

    // Verify target version belongs to this model
    if target_version.model_id != model_id {
        return Err(AppError::ValidationError(format!(
            "Version {} does not belong to model {}",
            target_version_id, model_id
        )));
    }

    // Verify artifact exists (optional but recommended)
    let artifact_exists = version_repo
        .verify_artifact_exists(&target_version_id)
        .await
        .unwrap_or(false);

    if !artifact_exists {
        add_log(
            &state.logs,
            "WARN",
            "Distillation",
            &format!(
                "Artifact path {} does not exist, proceeding anyway",
                target_version.artifact_path
            ),
        );
    }

    // Perform the rollback
    active_repo
        .set_active(&model_id, &target_version_id)
        .await?;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Rollback complete: model {} now active at version {}",
            model_id, target_version_id
        ),
    );

    Ok(RollbackResult {
        previous_version_id,
        rolled_back_to: target_version,
        backup_created,
    })
}

/// Get the list of previous versions for a model (for rollback UI)
#[tauri::command]
pub async fn distill_get_version_history(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    model_id: String,
    current_version_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<ModelVersion>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let version_repo = ModelVersionRepository::new(&db);

    match current_version_id {
        Some(vid) => {
            version_repo
                .get_previous_versions(&model_id, &vid, limit.unwrap_or(10))
                .await
        }
        None => version_repo.list_by_model(&model_id).await,
    }
}

// ============================================================================
// Evaluation Commands (Flow D - Evaluate + Compare)
// ============================================================================

#[tauri::command]
pub async fn distill_record_metric(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    metric: EvaluationMetricInput,
) -> Result<()> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = EvaluationMetricsRepository::new(&db);

    repo.upsert(&metric).await
}

#[tauri::command]
pub async fn distill_list_version_metrics(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    version_id: String,
) -> Result<Vec<EvaluationMetric>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = EvaluationMetricsRepository::new(&db);

    repo.list_for_version(&version_id).await
}

// ============================================================================
// Run Artifacts Commands
// ============================================================================

#[tauri::command]
pub async fn distill_record_artifact(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    artifact: RunArtifactInput,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Recording artifact {} for run {}",
            artifact.kind, artifact.run_id
        ),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = RunArtifactsRepository::new(&db);

    repo.insert(&artifact).await
}

#[tauri::command]
pub async fn distill_list_run_artifacts(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    run_id: String,
    kind: Option<String>,
) -> Result<Vec<RunArtifact>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let repo = RunArtifactsRepository::new(&db);

    match kind {
        Some(k) => repo.list_by_kind(&run_id, &k).await,
        None => repo.list_for_run(&run_id).await,
    }
}

// ============================================================================
// Backup Commands
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfoResponse {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub is_promotion_backup: bool,
}

impl From<BackupInfo> for BackupInfoResponse {
    fn from(info: BackupInfo) -> Self {
        Self {
            path: info.path.to_string_lossy().to_string(),
            file_name: info.file_name,
            size_bytes: info.size_bytes,
            is_promotion_backup: info.is_promotion_backup,
        }
    }
}

#[tauri::command]
pub async fn distill_create_backup(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    reason: Option<String>,
) -> Result<BackupInfoResponse> {
    add_log(&state.logs, "INFO", "Distillation", "Creating backup");

    let db_path = training_db_path(&app)?;
    let app_data_dir = resolve_app_data_dir(&app)?;
    let config = BackupConfig::new(&app_data_dir);

    let result = backup_training_db(&db_path, &config, reason.as_deref())?;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Backup created: {} ({} bytes)",
            result.backup_path.display(),
            result.size_bytes
        ),
    );

    Ok(BackupInfoResponse {
        path: result.backup_path.to_string_lossy().to_string(),
        file_name: result
            .backup_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        size_bytes: result.size_bytes,
        is_promotion_backup: false,
    })
}

#[tauri::command]
pub async fn distill_list_backups(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
) -> Result<Vec<BackupInfoResponse>> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    let config = BackupConfig::new(&app_data_dir);

    let backups = list_backups(&config)?;
    Ok(backups.into_iter().map(|b| b.into()).collect())
}

#[tauri::command]
pub async fn distill_restore_backup(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    backup_path: String,
) -> Result<BackupInfoResponse> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Restoring from backup: {}", backup_path),
    );

    let db_path = training_db_path(&app)?;
    let app_data_dir = resolve_app_data_dir(&app)?;
    let config = BackupConfig::new(&app_data_dir);
    let backup_path = PathBuf::from(backup_path);

    let pre_restore = restore_from_backup(&backup_path, &db_path, &config)?;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Backup restored. Pre-restore backup created at: {}",
            pre_restore.backup_path.display()
        ),
    );

    Ok(BackupInfoResponse {
        path: pre_restore.backup_path.to_string_lossy().to_string(),
        file_name: pre_restore
            .backup_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        size_bytes: pre_restore.size_bytes,
        is_promotion_backup: false,
    })
}

#[tauri::command]
pub async fn distill_cleanup_old_backups(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        "Cleaning up old backups",
    );

    let app_data_dir = resolve_app_data_dir(&app)?;
    let config = BackupConfig::new(&app_data_dir);

    let deleted = cleanup_old_backups(&config)?;
    let deleted_paths: Vec<String> = deleted
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Cleaned up {} old backups", deleted_paths.len()),
    );

    Ok(deleted_paths)
}

// ============================================================================
// Artifact Layout Commands
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLayoutInfo {
    pub root: String,
    pub models_base: String,
    pub models_versions: String,
    pub runs: String,
    pub evaluations: String,
}

#[tauri::command]
pub async fn distill_get_artifact_layout(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
) -> Result<ArtifactLayoutInfo> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);

    Ok(ArtifactLayoutInfo {
        root: layout.root().to_string_lossy().to_string(),
        models_base: layout.models_base_dir().to_string_lossy().to_string(),
        models_versions: layout.models_versions_dir().to_string_lossy().to_string(),
        runs: layout.runs_dir().to_string_lossy().to_string(),
        evaluations: layout.evaluations_dir().to_string_lossy().to_string(),
    })
}

// ============================================================================
// Evaluation Orchestrator (Rust -> Python evaluator)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillEvalConfig {
    pub version_id: String,
    pub dataset_id: String,
    pub eval_id: Option<String>,
    pub max_samples: Option<i64>,
    pub max_new_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub seed: Option<i64>,
    pub compute_teacher_agreement: Option<bool>,
}

#[tauri::command]
pub async fn distill_evaluate_version(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    config: DistillEvalConfig,
) -> Result<String> {
    let version_id = config.version_id.trim().to_string();
    let dataset_id = config.dataset_id.trim().to_string();
    if version_id.is_empty() || dataset_id.is_empty() {
        return Err(crate::domain::error::AppError::ValidationError(
            "version_id and dataset_id are required".to_string(),
        ));
    }

    let eval_id = config
        .eval_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Starting evaluation {} for version {}", eval_id, version_id),
    );

    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);
    layout.ensure()?;
    let eval_dir = layout.evaluation_dir(&eval_id);
    std::fs::create_dir_all(&eval_dir).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to create eval_dir {}: {e}",
            eval_dir.display()
        ))
    })?;

    let script_path: PathBuf = app
        .path()
        .resolve("resources/scripts/distill-eval.py", BaseDirectory::Resource)
        .or_else(|_| {
            // Try development path (src-tauri/resources)
            let cwd = std::env::current_dir().map_err(|e| {
                crate::domain::error::AppError::Internal(format!("Failed to resolve cwd: {e}"))
            })?;
            let dev_path = cwd
                .join("src-tauri")
                .join("resources")
                .join("scripts")
                .join("distill-eval.py");
            if dev_path.exists() {
                return Ok::<PathBuf, crate::domain::error::AppError>(dev_path);
            }

            // Try target/debug path (for dev builds)
            let debug_path = cwd
                .join("target")
                .join("debug")
                .join("resources")
                .join("scripts")
                .join("distill-eval.py");
            if debug_path.exists() {
                return Ok::<PathBuf, crate::domain::error::AppError>(debug_path);
            }

            // Try relative to exe (for some dev setups)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let exe_relative = exe_dir.join("resources/scripts/distill-eval.py");
                    if exe_relative.exists() {
                        return Ok::<PathBuf, crate::domain::error::AppError>(exe_relative);
                    }
                }
            }

            Err(crate::domain::error::AppError::NotFound(
                "Python evaluator script not found in dev or debug paths".to_string(),
            ))
        })
        .map_err(|e| {
            crate::domain::error::AppError::NotFound(format!(
                "Python evaluator script not found: {e}"
            ))
        })?;

    // Strip Windows extended path prefix (\\?\) that Python can't handle
    let script_path = {
        let path_str = script_path.to_string_lossy();
        if path_str.starts_with(r"\\?\") {
            PathBuf::from(&path_str[4..])
        } else {
            script_path
        }
    };

    let db_path = training_db_path(&app)?;
    let cfg_json = serde_json::json!({
        "eval_id": eval_id,
        "version_id": version_id,
        "dataset_id": dataset_id,
        "run_dir": eval_dir.to_string_lossy(),
        "training_db_path": db_path.to_string_lossy(),
        "max_samples": config.max_samples,
        "max_new_tokens": config.max_new_tokens,
        "temperature": config.temperature,
        "top_p": config.top_p,
        "seed": config.seed,
        "compute_teacher_agreement": config.compute_teacher_agreement.unwrap_or(false),
    });

    let config_path = eval_dir.join("eval_config.json");
    if let Err(e) = std::fs::write(&config_path, serde_json::to_vec_pretty(&cfg_json).unwrap()) {
        return Err(crate::domain::error::AppError::Internal(format!(
            "Failed to write eval_config.json {}: {e}",
            config_path.display()
        )));
    }

    let stdout_log_path = eval_dir.join("eval_stdout.log");
    let stderr_log_path = eval_dir.join("eval_stderr.log");
    let metrics_log_path = eval_dir.join("eval_metrics.jsonl");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_log_path);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_log_path);

    // Spawn python with --config only (no --stdin to avoid blocking on stdin.read()).
    let mut cmd = TokioCommand::new("python");

    // Convert paths to strings explicitly to avoid PathBuf arg issues
    let script_path_str = script_path.to_string_lossy().to_string();
    let config_path_str = config_path.to_string_lossy().to_string();

    cmd.arg(&script_path_str)
        .arg("--config")
        .arg(&config_path_str)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to spawn evaluator: {e}"))
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        crate::domain::error::AppError::Internal("Evaluator stdout unavailable".to_string())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        crate::domain::error::AppError::Internal("Evaluator stderr unavailable".to_string())
    })?;

    let child = Arc::new(AsyncMutex::new(child));

    let last_error: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));

    let app_clone = app.clone();
    let db_path_out = db_path.clone();
    let version_id_out = version_id.clone();
    let dataset_id_out = dataset_id.clone();
    let stdout_log_path_out = stdout_log_path.clone();
    let metrics_log_path_out = metrics_log_path.clone();
    let last_error_out = last_error.clone();
    tauri::async_runtime::spawn(async move {
        let db_out = match TrainingDb::connect(&db_path_out).await {
            Ok(db) => db,
            Err(_) => return,
        };
        let metrics_repo = EvaluationMetricsRepository::new(&db_out);

        let mut stdout_log = match TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&stdout_log_path_out)
            .await
        {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut metrics_log = TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&metrics_log_path_out)
            .await
            .ok();

        let mut lines = TokioBufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stdout_log.write_all(line.as_bytes()).await;
            let _ = stdout_log.write_all(b"\n").await;

            let Ok(msg) = serde_json::from_str::<DistillPythonMessage>(&line) else {
                continue;
            };

            let _ = app_clone.emit("distill-eval-stream", msg.clone());

            match msg.kind.as_str() {
                "metric" => {
                    if let Some(ref mut file) = metrics_log {
                        let _ = file.write_all(line.as_bytes()).await;
                        let _ = file.write_all(b"\n").await;
                    }

                    let payload = &msg.payload;
                    let name = payload.get("name").and_then(|v| v.as_str());
                    let value = payload.get("value").and_then(|v| v.as_f64());
                    if let (Some(name), Some(value)) = (name, value) {
                        let _ = metrics_repo
                            .upsert(&EvaluationMetricInput {
                                version_id: version_id_out.clone(),
                                dataset_id: dataset_id_out.clone(),
                                metric_name: name.to_string(),
                                metric_value: value,
                            })
                            .await;
                    }
                }
                "status" => {
                    let payload = &msg.payload;
                    let level = payload.get("level").and_then(|v| v.as_str()).unwrap_or("");
                    let message = payload
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if level.eq_ignore_ascii_case("error") {
                        let mut guard = last_error_out.lock().unwrap();
                        *guard = Some(message);
                    }
                }
                _ => {}
            }
        }
    });

    let stderr_log_path_err = stderr_log_path.clone();
    let last_error_err = last_error.clone();
    tauri::async_runtime::spawn(async move {
        let mut stderr_log = match TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_log_path_err)
            .await
        {
            Ok(f) => f,
            Err(_) => return,
        };

        let mut lines = TokioBufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_log.write_all(line.as_bytes()).await;
            let _ = stderr_log.write_all(b"\n").await;

            if !line.trim().is_empty() {
                let mut guard = last_error_err.lock().unwrap();
                if guard.is_none() {
                    let trimmed = if line.len() > 400 {
                        format!("{}...", &line[..400])
                    } else {
                        line.clone()
                    };
                    *guard = Some(trimmed);
                }
            }
        }
    });

    let app_wait = app.clone();
    let last_error_wait = last_error.clone();
    let eval_id_wait = eval_id.clone();
    let child_wait = child.clone();
    tauri::async_runtime::spawn(async move {
        let status = {
            let mut guard = child_wait.lock().await;
            guard.wait().await
        };

        let failed = status.map(|s| !s.success()).unwrap_or(true);
        if failed {
            let guard = last_error_wait.lock().unwrap();
            let msg = guard
                .clone()
                .unwrap_or_else(|| "Evaluator exited with failure".to_string());
            let _ = app_wait.emit(
                "distill-eval-stream",
                DistillPythonMessage {
                    kind: "status".to_string(),
                    payload: serde_json::json!({
                        "level": "error",
                        "message": msg,
                        "eval_id": eval_id_wait,
                    }),
                },
            );
        } else {
            let _ = app_wait.emit(
                "distill-eval-stream",
                DistillPythonMessage {
                    kind: "status".to_string(),
                    payload: serde_json::json!({
                        "level": "info",
                        "message": "evaluation completed",
                        "eval_id": eval_id_wait,
                    }),
                },
            );
        }
    });

    Ok(eval_id)
}

// ============================================================================
// Python Orchestrator (Rust -> Python runner)
// ============================================================================

struct DistillTrainerLaunchGuard {
    state: Arc<AppState>,
    run_id: String,
    active: bool,
}

impl DistillTrainerLaunchGuard {
    fn reserve(state: Arc<AppState>, run_id: &str) -> Result<Self> {
        // Lock, check, and release within a block to avoid holding borrows when moving state
        {
            let trainers = state.distill_trainers.lock().unwrap();
            if trainers.contains_key(run_id) {
                return Err(crate::domain::error::AppError::ValidationError(format!(
                    "Trainer already running for run_id={}",
                    run_id
                )));
            }
        }

        {
            let mut launches = state.distill_trainer_launches.lock().unwrap();
            if launches.contains(run_id) {
                return Err(crate::domain::error::AppError::ValidationError(format!(
                    "Trainer already starting for run_id={}",
                    run_id
                )));
            }
            launches.insert(run_id.to_string());
        }

        Ok(Self {
            state,
            run_id: run_id.to_string(),
            active: true,
        })
    }

    fn insert_handle(&mut self, handle: DistillTrainerHandle) -> Result<()> {
        let mut trainers = self.state.distill_trainers.lock().unwrap();
        let mut launches = self.state.distill_trainer_launches.lock().unwrap();

        if trainers.contains_key(&self.run_id) {
            return Err(crate::domain::error::AppError::ValidationError(format!(
                "Trainer already running for run_id={}",
                self.run_id
            )));
        }

        trainers.insert(self.run_id.clone(), handle);
        launches.remove(&self.run_id);
        self.active = false;
        Ok(())
    }
}

impl Drop for DistillTrainerLaunchGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let mut launches = self.state.distill_trainer_launches.lock().unwrap();
        launches.remove(&self.run_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillTrainConfig {
    pub run_id: String,
    pub run_dir: String,
    pub mode: Option<String>,
    pub seed: Option<i64>,
    pub steps: Option<i64>,
    pub emit_every: Option<i64>,
    pub hyperparams: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillPythonMessage {
    pub kind: String,
    pub payload: JsonValue,
}

#[tauri::command]
pub async fn distill_start_python_training(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    config: DistillTrainConfig,
) -> Result<String> {
    let run_id = config.run_id.trim().to_string();
    if run_id.is_empty() {
        return Err(crate::domain::error::AppError::ValidationError(
            "run_id is required".to_string(),
        ));
    }

    // Prevent duplicate trainer launches for the same run_id (running or still starting).
    let mut launch_guard = DistillTrainerLaunchGuard::reserve(state.inner().clone(), &run_id)?;

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Starting python trainer for run {}", run_id),
    );

    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);
    layout.ensure()?;

    // Force run_dir to be under our artifact layout, unless caller already provided one.
    let run_dir = if config.run_dir.trim().is_empty() {
        layout.run_dir(&run_id)
    } else {
        PathBuf::from(&config.run_dir)
    };
    std::fs::create_dir_all(&run_dir).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to create run_dir {}: {e}",
            run_dir.display()
        ))
    })?;

    // Clear any prior cancellation marker.
    let cancel_flag_path = run_dir.join("cancel.flag");
    let _ = std::fs::remove_file(&cancel_flag_path);

    // Resolve the python script.
    // In production, resources are bundled. In dev, they're in target/debug/resources/
    let script_path: PathBuf = app
        .path()
        .resolve("resources/scripts/distill-train.py", BaseDirectory::Resource)
        .or_else(|_| {
            // Try development path (src-tauri/resources)
            let cwd = std::env::current_dir().map_err(|e| {
                crate::domain::error::AppError::Internal(format!("Failed to resolve cwd: {e}"))
            })?;
            let dev_path = cwd
                .join("src-tauri")
                .join("resources")
                .join("scripts")
                .join("distill-train.py");
            if dev_path.exists() {
                return Ok::<PathBuf, crate::domain::error::AppError>(dev_path);
            }

            // Try target/debug path (for dev builds)
            let debug_path = cwd
                .join("target")
                .join("debug")
                .join("resources")
                .join("scripts")
                .join("distill-train.py");
            if debug_path.exists() {
                return Ok::<PathBuf, crate::domain::error::AppError>(debug_path);
            }

            // Try relative to exe (for some dev setups)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let exe_relative = exe_dir.join("resources/scripts/distill-train.py");
                    if exe_relative.exists() {
                        return Ok::<PathBuf, crate::domain::error::AppError>(exe_relative);
                    }
                }
            }

            Err(crate::domain::error::AppError::NotFound(
                "Python trainer script not found in dev or debug paths".to_string(),
            ))
        })
        .map_err(|e| {
            crate::domain::error::AppError::NotFound(format!(
                "Python trainer script not found: {e}"
            ))
        })?;

    // Strip Windows extended path prefix (\\?\) that Python can't handle
    let script_path = {
        let path_str = script_path.to_string_lossy();
        if path_str.starts_with(r"\\?\") {
            PathBuf::from(&path_str[4..])
        } else {
            script_path
        }
    };

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let run_repo = TrainingRunRepository::new(&db);
    // Ensure run exists before launching.
    let _run = run_repo.get(&run_id).await?;

    // Write config.json so python can be launched in file-mode deterministically.
    let config_path = run_dir.join("trainer_config.json");
    let cfg_json = serde_json::json!({
        "run_id": run_id.clone(),
        "run_dir": run_dir.to_string_lossy(),
        "mode": config
            .mode
            .clone()
            .unwrap_or_else(|| "fine_tune".to_string()),
        "seed": config.seed,
        "steps": config.steps.unwrap_or(100),
        "emit_every": config.emit_every.unwrap_or(1),
        "training_db_path": db_path.to_string_lossy(),
        "dataset_source": "db",
        "hyperparams": config.hyperparams.clone(),
    });
    if let Err(e) = std::fs::write(&config_path, serde_json::to_vec_pretty(&cfg_json).unwrap()) {
        let err = crate::domain::error::AppError::Internal(format!(
            "Failed to write trainer_config.json {}: {e}",
            config_path.display()
        ));
        let _ = run_repo
            .set_status(
                &run_id,
                TrainingStatus::Failed,
                Some(chrono::Utc::now().to_rfc3339()),
                Some(err.to_string()),
            )
            .await;
        return Err(err);
    }

    let stdout_log_path = run_dir.join("trainer_stdout.log");
    let stderr_log_path = run_dir.join("trainer_stderr.log");
    let metrics_log_path = run_dir.join("trainer_metrics.jsonl");
    // Touch log files early so the UI/DB can reference paths deterministically.
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_log_path);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_log_path);

    // Record baseline run artifacts (best-effort).
    let artifacts_repo = RunArtifactsRepository::new(&db);
    let _ = artifacts_repo
        .insert(&RunArtifactInput {
            artifact_id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            kind: "config".to_string(),
            path: config_path.to_string_lossy().to_string(),
            hash: None,
            size_bytes: None,
        })
        .await;
    let _ = artifacts_repo
        .insert(&RunArtifactInput {
            artifact_id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            kind: "log".to_string(),
            path: stdout_log_path.to_string_lossy().to_string(),
            hash: None,
            size_bytes: None,
        })
        .await;
    let _ = artifacts_repo
        .insert(&RunArtifactInput {
            artifact_id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            kind: "log".to_string(),
            path: stderr_log_path.to_string_lossy().to_string(),
            hash: None,
            size_bytes: None,
        })
        .await;

    // Spawn python with --config only (no --stdin to avoid blocking on stdin.read()).
    let mut cmd = TokioCommand::new("python");

    // Log the exact command for debugging
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!(
            "Python command: python {} --config {}",
            script_path.display(),
            config_path.display()
        ),
    );

    // Convert paths to strings explicitly to avoid PathBuf arg issues
    let script_path_str = script_path.to_string_lossy().to_string();
    let config_path_str = config_path.to_string_lossy().to_string();

    cmd.arg(&script_path_str)
        .arg("--config")
        .arg(&config_path_str)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let err =
                crate::domain::error::AppError::Internal(format!("Failed to spawn python: {e}"));
            let _ = run_repo
                .set_status(
                    &run_id,
                    TrainingStatus::Failed,
                    Some(chrono::Utc::now().to_rfc3339()),
                    Some(err.to_string()),
                )
                .await;
            return Err(err);
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let err =
                crate::domain::error::AppError::Internal("Trainer stdout unavailable".to_string());
            let _ = child.kill().await;
            let _ = run_repo
                .set_status(
                    &run_id,
                    TrainingStatus::Failed,
                    Some(chrono::Utc::now().to_rfc3339()),
                    Some(err.to_string()),
                )
                .await;
            return Err(err);
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let err =
                crate::domain::error::AppError::Internal("Trainer stderr unavailable".to_string());
            let _ = child.kill().await;
            let _ = run_repo
                .set_status(
                    &run_id,
                    TrainingStatus::Failed,
                    Some(chrono::Utc::now().to_rfc3339()),
                    Some(err.to_string()),
                )
                .await;
            return Err(err);
        }
    };

    let child = Arc::new(AsyncMutex::new(child));

    if let Err(e) = launch_guard.insert_handle(DistillTrainerHandle {
        child: child.clone(),
        run_id: run_id.clone(),
        run_dir: run_dir.clone(),
    }) {
        let child_to_kill = child.clone();
        tauri::async_runtime::spawn(async move {
            let mut guard = child_to_kill.lock().await;
            let _ = guard.kill().await;
        });
        return Err(e);
    }

    // Mark run as running (now that the process is registered).
    if let Err(e) = run_repo
        .set_status(&run_id, TrainingStatus::Running, None, None)
        .await
    {
        add_log(
            &state.logs,
            "ERROR",
            "Distillation",
            &format!("Failed to mark run as running: {e}"),
        );
    }

    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!(
            "Python process started. Config: {}, Script: {}",
            config_path.display(),
            script_path.display()
        ),
    );

    let last_error: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));

    // Clone state.logs for use in async tasks
    let logs_for_stdout = state.logs.clone();
    let logs_for_stderr = state.logs.clone();

    // Clone app for stderr event emission
    let app_for_stderr = app.clone();

    // Emit stdout JSONL to the frontend as `distill-train-stream`.
    let app_clone = app.clone();
    let run_id_out = run_id.clone();
    let db_path_out = db_path.clone();
    let stdout_log_path_out = stdout_log_path.clone();
    let metrics_log_path_out = metrics_log_path.clone();
    let run_dir_out = run_dir.clone();
    let last_error_out = last_error.clone();
    tauri::async_runtime::spawn(async move {
        // Create a fresh DB connection for this task
        let db_out = match TrainingDb::connect(&db_path_out).await {
            Ok(db) => db,
            Err(_) => return,
        };
        let log_repo = TrainingLogRepository::new(&db_out);
        let run_repo = TrainingRunRepository::new(&db_out);
        let artifacts_repo = RunArtifactsRepository::new(&db_out);

        let mut stdout_log = match TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&stdout_log_path_out)
            .await
        {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut metrics_log = TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&metrics_log_path_out)
            .await
            .ok();

        let mut lines = TokioBufReader::new(stdout).lines();
        let mut first_line = true;
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stdout_log.write_all(line.as_bytes()).await;
            let _ = stdout_log.write_all(b"\n").await;

            // Log the first few lines for debugging
            if first_line {
                first_line = false;
                add_log(
                    &logs_for_stdout,
                    "DEBUG",
                    "Distillation",
                    &format!("Python stdout first line: {}", if line.len() > 200 { &line[..200] } else { &line }),
                );
            }

            let Ok(msg) = serde_json::from_str::<DistillPythonMessage>(&line) else {
                // Log non-JSON lines for debugging
                if !line.trim().is_empty() {
                    add_log(
                        &logs_for_stdout,
                        "DEBUG",
                        "Distillation",
                        &format!("Python stdout (non-JSON): {}", if line.len() > 100 { &line[..100] } else { &line }),
                    );
                }
                continue;
            };

            // Log status and error messages
            if msg.kind == "status" {
                let level = msg.payload.get("level").and_then(|v| v.as_str()).unwrap_or("info");
                let message = msg.payload.get("message").and_then(|v| v.as_str()).unwrap_or("");
                add_log(
                    &logs_for_stdout,
                    if level == "error" { "ERROR" } else if level == "warn" { "WARN" } else { "INFO" },
                    "Distillation",
                    &format!("Python: {}", message),
                );
            }

            let _ = app_clone.emit("distill-train-stream", msg.clone());

            match msg.kind.as_str() {
                "progress" => {
                    let payload = &msg.payload;
                    let epoch = payload.get("epoch").and_then(|v| v.as_i64()).unwrap_or(0);
                    let step = match payload.get("step").and_then(|v| v.as_i64()) {
                        Some(v) => v,
                        None => continue,
                    };
                    let loss = payload.get("loss").and_then(|v| v.as_f64());
                    let lr = payload.get("lr").and_then(|v| v.as_f64());
                    let temperature = payload.get("temperature").and_then(|v| v.as_f64());

                    let resources = payload.get("resources").and_then(|v| v.as_object());
                    let cpu_util = resources
                        .and_then(|o| o.get("cpu_percent"))
                        .and_then(|v| v.as_f64());
                    let ram_usage_mb = resources
                        .and_then(|o| o.get("ram_rss_bytes"))
                        .and_then(|v| v.as_i64())
                        .map(|b| b / (1024 * 1024));
                    let gpu_util = resources
                        .and_then(|o| o.get("gpu_util_percent"))
                        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));

                    let _ = log_repo
                        .insert(&TrainingLogInput {
                            run_id: run_id_out.clone(),
                            epoch,
                            step,
                            loss,
                            lr,
                            temperature,
                            cpu_util,
                            ram_usage_mb,
                            gpu_util,
                        })
                        .await;
                }
                "metric" => {
                    if let Some(ref mut file) = metrics_log {
                        let _ = file.write_all(line.as_bytes()).await;
                        let _ = file.write_all(b"\n").await;
                    }
                }
                "artifact" => {
                    let payload = &msg.payload;
                    let kind = payload.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");

                    let db_kind = match kind {
                        "config" | "log" | "checkpoint" | "adapter" | "merged_model" | "gguf" => {
                            Some(kind)
                        }
                        // Older/compat kinds emitted by the python runner.
                        "model" => Some("merged_model"),
                        "result" => Some("log"),
                        _ => None,
                    };

                    if let (Some(db_kind), false) = (db_kind, path.trim().is_empty()) {
                        let _ = artifacts_repo
                            .insert(&RunArtifactInput {
                                artifact_id: Uuid::new_v4().to_string(),
                                run_id: run_id_out.clone(),
                                kind: db_kind.to_string(),
                                path: path.to_string(),
                                hash: None,
                                size_bytes: None,
                            })
                            .await;
                    }
                }
                "dataset" => {
                    let payload = &msg.payload;
                    if let Some(path) = payload.get("path").and_then(|v| v.as_str()) {
                        let _ = artifacts_repo
                            .insert(&RunArtifactInput {
                                artifact_id: Uuid::new_v4().to_string(),
                                run_id: run_id_out.clone(),
                                kind: "config".to_string(),
                                path: path.to_string(),
                                hash: None,
                                size_bytes: None,
                            })
                            .await;
                    }
                }
                "status" => {
                    let payload = &msg.payload;
                    let level = payload.get("level").and_then(|v| v.as_str()).unwrap_or("");
                    let message = payload
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if level.eq_ignore_ascii_case("error") {
                        {
                            let mut guard = last_error_out.lock().unwrap();
                            *guard = Some(message.clone());
                        }
                        if let Some(trace) = payload.get("trace").and_then(|v| v.as_str()) {
                            let trace_path = run_dir_out.join("trainer_error.trace");
                            let _ = tokio::fs::write(trace_path, trace).await;
                        }
                        // Update DB early (end_time set by exit monitor).
                        let _ = run_repo
                            .set_status(&run_id_out, TrainingStatus::Failed, None, Some(message))
                            .await;
                    }
                }
                _ => {}
            }
        }
    });

    // Capture stderr to file and log important messages to app.
    let stderr_log_path_err = stderr_log_path.clone();
    let last_error_err = last_error.clone();
    tauri::async_runtime::spawn(async move {
        let mut stderr_log = match TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_log_path_err)
            .await
        {
            Ok(f) => f,
            Err(_) => return,
        };

        let mut lines = TokioBufReader::new(stderr).lines();
        let mut stderr_line_count = 0;
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_log.write_all(line.as_bytes()).await;
            let _ = stderr_log.write_all(b"\n").await;

            // Log first few stderr lines for debugging
            stderr_line_count += 1;
            if stderr_line_count <= 5 && !line.trim().is_empty() {
                add_log(
                    &logs_for_stderr,
                    "WARN",
                    "Distillation",
                    &format!("Python stderr: {}", if line.len() > 500 { &line[..500] } else { &line }),
                );
            }

            // Emit stderr to frontend for live display
            if !line.trim().is_empty() {
                // Store last error for exit handling
                let mut guard = last_error_err.lock().unwrap();
                if guard.is_none() {
                    let trimmed = if line.len() > 400 {
                        format!("{}...", &line[..400])
                    } else {
                        line.clone()
                    };
                    *guard = Some(trimmed);
                }
                drop(guard);

                // Emit stderr as event to frontend
                let msg = serde_json::json!({
                    "kind": "stderr",
                    "payload": {
                        "message": line,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }
                });
                let _ = app_for_stderr.emit("distill-train-stream", msg);
            }
        }
    });

    // Monitor exit without holding the child mutex across await (so cancel can kill).
    let state_clone = state.inner().clone();
    let run_id_wait = run_id.clone();
    let run_dir_wait = run_dir.clone();
    let last_error_wait = last_error.clone();
    let child_wait = child.clone();
    let db_path_wait = db_path.clone();
    let app_wait = app.clone();
    tauri::async_runtime::spawn(async move {
        // Create a fresh DB connection for this task
        let db_wait = match TrainingDb::connect(&db_path_wait).await {
            Ok(db) => db,
            Err(_) => return,
        };
        let run_repo = TrainingRunRepository::new(&db_wait);
        let version_repo = ModelVersionRepository::new(&db_wait);
        let artifacts_repo = RunArtifactsRepository::new(&db_wait);
        loop {
            let exited = {
                let mut guard = child_wait.lock().await;
                match guard.try_wait() {
                    Ok(Some(status)) => Some(Ok(status)),
                    Ok(None) => None,
                    Err(err) => Some(Err(err)),
                }
            };

            let outcome = match exited {
                None => {
                    sleep(Duration::from_millis(250)).await;
                    continue;
                }
                Some(v) => v,
            };

            let cancelled = run_dir_wait.join("cancel.flag").exists();
            let end_time = Some(chrono::Utc::now().to_rfc3339());

            match outcome {
                Ok(status) => {
                    let final_status = if cancelled {
                        TrainingStatus::Cancelled
                    } else if status.success() {
                        TrainingStatus::Completed
                    } else {
                        TrainingStatus::Failed
                    };

                    let failure_reason = if matches!(final_status, TrainingStatus::Failed) {
                        let guard = last_error_wait.lock().unwrap();
                        guard
                            .clone()
                            .or_else(|| Some(format!("Trainer exited: {}", status)))
                    } else {
                        None
                    };

                    let is_completed = matches!(final_status, TrainingStatus::Completed);
                    let _ = run_repo
                        .set_status(&run_id_wait, final_status, end_time, failure_reason)
                        .await;

                    if is_completed {
                        if let Ok(existing) = version_repo.find_by_run_id(&run_id_wait).await {
                            if existing.is_none() {
                                if let Ok(run) = run_repo.get(&run_id_wait).await {
                                    if let Ok(artifacts) =
                                        artifacts_repo.list_for_run(&run_id_wait).await
                                    {
                                        let preferred = ["merged_model", "adapter", "gguf"];
                                        let mut picked: Option<&RunArtifact> = None;
                                        for kind in preferred {
                                            if let Some(found) =
                                                artifacts.iter().find(|a| a.kind == kind)
                                            {
                                                picked = Some(found);
                                                break;
                                            }
                                        }

                                        if let Some(artifact) = picked {
                                            let version_id = Uuid::new_v4().to_string();
                                            let input = ModelVersionInput {
                                                version_id: version_id.clone(),
                                                model_id: run.student_model_id.clone(),
                                                run_id: Some(run_id_wait.clone()),
                                                parent_version_id: run.base_version_id.clone(),
                                                artifact_path: artifact.path.clone(),
                                                artifact_hash: artifact.hash.clone(),
                                                artifact_size_bytes: artifact.size_bytes,
                                                notes: Some(format!(
                                                    "Auto-created from run {}",
                                                    run_id_wait
                                                )),
                                            };
                                            if let Err(e) = version_repo.insert(&input).await {
                                                add_log(
                                                    &state_clone.logs,
                                                    "ERROR",
                                                    "Distillation",
                                                    &format!(
                                                        "Failed to auto-create model version for run {}: {e}",
                                                        run_id_wait
                                                    ),
                                                );
                                            } else {
                                                add_log(
                                                    &state_clone.logs,
                                                    "INFO",
                                                    "Distillation",
                                                    &format!(
                                                        "Auto-created model version {} for run {}",
                                                        version_id, run_id_wait
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    let failure = {
                        let guard = last_error_wait.lock().unwrap();
                        guard
                            .clone()
                            .unwrap_or_else(|| format!("Trainer wait failed: {}", err))
                    };
                    let _ = run_repo
                        .set_status(
                            &run_id_wait,
                            TrainingStatus::Failed,
                            end_time,
                            Some(failure),
                        )
                        .await;
                }
            }

            {
                let mut guard = state_clone.distill_trainers.lock().unwrap();
                guard.remove(&run_id_wait);
            }

            let _ = app_wait.emit(
                "distill-train-stream",
                DistillPythonMessage {
                    kind: "status".to_string(),
                    payload: serde_json::json!({
                        "level": "info",
                        "message": "trainer exited",
                        "run_id": run_id_wait,
                        "cancelled": cancelled
                    }),
                },
            );
            break;
        }
    });

    // Detach (return a process id token the UI can display).
    let token = format!("{}:{}", run_id, Uuid::new_v4());
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Python trainer spawned (token={})", token),
    );

    Ok(token)
}

#[tauri::command]
pub async fn distill_cancel_python_training(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
) -> Result<()> {
    let run_id = run_id.trim().to_string();
    if run_id.is_empty() {
        return Err(crate::domain::error::AppError::ValidationError(
            "run_id is required".to_string(),
        ));
    }

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Cancelling python trainer for run {}", run_id),
    );

    let (child, run_dir) = {
        let guard = state.distill_trainers.lock().unwrap();
        let handle = guard.get(&run_id).ok_or_else(|| {
            crate::domain::error::AppError::ValidationError(format!(
                "No active trainer for run_id={}",
                run_id
            ))
        })?;
        (handle.child.clone(), handle.run_dir.clone())
    };

    let cancel_flag_path = run_dir.join("cancel.flag");
    tokio::fs::write(&cancel_flag_path, b"cancel\n")
        .await
        .map_err(|e| {
            crate::domain::error::AppError::Internal(format!(
                "Failed to write cancel flag {}: {e}",
                cancel_flag_path.display()
            ))
        })?;

    // Update DB immediately (final end_time will be set by exit monitor).
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let run_repo = TrainingRunRepository::new(&db);
    let _ = run_repo
        .set_status(
            &run_id,
            TrainingStatus::Cancelled,
            Some(chrono::Utc::now().to_rfc3339()),
            None,
        )
        .await;

    // Give the python loop a brief chance to exit gracefully, then force-kill if needed.
    tauri::async_runtime::spawn(async move {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let exited = {
                let mut guard = child.lock().await;
                match guard.try_wait() {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(_) => true,
                }
            };
            if exited {
                break;
            }
            if Instant::now() >= deadline {
                let mut guard = child.lock().await;
                let _ = guard.kill().await;
                break;
            }
            sleep(Duration::from_millis(250)).await;
        }
    });

    Ok(())
}

// ============================================================================
// Soft Labels Commands (Phase 1: Data Preparation)
// ============================================================================

#[tauri::command]
pub async fn distill_generate_soft_labels(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: SoftLabelGenerationInput,
) -> Result<SoftLabelGenerationResult> {
    use sha2::{Digest, Sha256};

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Generating soft labels for {} prompts", input.prompts.len()),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let sl_repo = SoftLabelRepository::new(&db);
    let model_repo = ModelRepository::new(&db);

    // Get teacher model info
    let teacher_model = model_repo.get(&input.teacher_model_id).await.map_err(|e| {
        crate::domain::error::AppError::NotFound(format!(
            "Teacher model not found: {} - {}",
            input.teacher_model_id, e
        ))
    })?;

    let mut soft_label_ids = Vec::new();
    let mut cached_count = 0;
    let mut generated_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for prompt in &input.prompts {
        // Compute prompt hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        let prompt_hash = format!("{:x}", hasher.finalize());

        // Check if soft labels already exist for this prompt + teacher
        match sl_repo
            .get_by_prompt_and_teacher(&prompt_hash, &input.teacher_model_id)
            .await
        {
            Ok(Some(existing)) => {
                soft_label_ids.push(existing.soft_label_id.clone());
                cached_count += 1;
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                errors.push(format!(
                    "Failed to check cache for prompt '{}': {}",
                    &prompt.chars().take(50).collect::<String>(),
                    e
                ));
                failed_count += 1;
                continue;
            }
        }

        // Generate new soft labels
        let soft_label_id = Uuid::new_v4().to_string();

        // Call teacher model based on provider
        let teacher_output = match teacher_model.provider.as_str() {
            "api" => {
                // For API teachers, we can only get text output (no logits)
                // We'll store as "one_hot" or "text_only" type
                match call_teacher_api(&state, &teacher_model, prompt, input.temperature).await {
                    Ok(output) => output,
                    Err(e) => {
                        errors.push(format!(
                            "Failed to call teacher API for prompt '{}': {}",
                            &prompt.chars().take(50).collect::<String>(),
                            e
                        ));
                        failed_count += 1;
                        continue;
                    }
                }
            }
            "local" => {
                // For local teachers, we could potentially get logits
                // For now, we'll also call as text-only (logits generation requires Python integration)
                match call_teacher_local(&state, &teacher_model, prompt, input.temperature).await {
                    Ok(output) => output,
                    Err(e) => {
                        errors.push(format!(
                            "Failed to call local teacher for prompt '{}': {}",
                            &prompt.chars().take(50).collect::<String>(),
                            e
                        ));
                        failed_count += 1;
                        continue;
                    }
                }
            }
            _ => {
                errors.push(format!(
                    "Unsupported teacher provider: {}",
                    teacher_model.provider
                ));
                failed_count += 1;
                continue;
            }
        };

        // Determine soft label type based on teacher provider
        let soft_label_type = if teacher_model.provider == "local" {
            // For local teachers, we could potentially store logits
            // For now, use "one_hot" as placeholder
            "one_hot".to_string()
        } else {
            // For API teachers, we only have text output
            "text_only".to_string()
        };

        // Create soft label input
        let sl_input = SoftLabelInput {
            soft_label_id: soft_label_id.clone(),
            prompt: prompt.clone(),
            prompt_hash,
            teacher_model_id: input.teacher_model_id.clone(),
            teacher_output: teacher_output.clone(),
            soft_label_type,
            soft_labels_blob: None, // Will be populated by Python script for local teachers
            temperature: input.temperature,
            metadata_json: Some(
                serde_json::json!({
                    "generated_at": chrono::Utc::now().to_rfc3339(),
                    "teacher_provider": teacher_model.provider,
                    "teacher_display_name": teacher_model.display_name,
                })
                .to_string(),
            ),
        };

        // Save to database
        if let Err(e) = sl_repo.insert(&sl_input).await {
            errors.push(format!(
                "Failed to save soft label for prompt '{}': {}",
                &prompt.chars().take(50).collect::<String>(),
                e
            ));
            failed_count += 1;
            continue;
        }

        soft_label_ids.push(soft_label_id);
        generated_count += 1;
    }

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Soft label generation complete: {} cached, {} generated, {} failed",
            cached_count, generated_count, failed_count
        ),
    );

    Ok(SoftLabelGenerationResult {
        soft_label_ids,
        cached_count,
        generated_count,
        failed_count,
        errors,
    })
}

/// Helper function to call teacher API (OpenAI, Gemini, etc.)
async fn call_teacher_api(
    state: &Arc<AppState>,
    teacher_model: &Model,
    prompt: &str,
    temperature: f64,
) -> Result<String> {
    // TODO: Implement actual API call using state.llm_client
    // For now, return a placeholder to satisfy the type system
    Ok(format!(
        "[Teacher API Response for: {}]",
        &prompt.chars().take(30).collect::<String>()
    ))
}

/// Helper function to call local teacher model
async fn call_teacher_local(
    state: &Arc<AppState>,
    teacher_model: &Model,
    prompt: &str,
    temperature: f64,
) -> Result<String> {
    // TODO: Implement actual local model call
    // For now, return a placeholder to satisfy the type system
    Ok(format!(
        "[Local Teacher Response for: {}]",
        &prompt.chars().take(30).collect::<String>()
    ))
}

#[tauri::command]
pub async fn distill_list_soft_labels(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    teacher_model_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<SoftLabel>> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;

    if let Some(teacher_id) = teacher_model_id {
        // List soft labels for a specific teacher
        let rows = sqlx::query_as::<
            _,
            crate::infrastructure::db::training::repositories::SoftLabelEntity,
        >(
            "SELECT soft_label_id, prompt, prompt_hash, teacher_model_id, teacher_output, \
             soft_label_type, temperature, metadata_json, created_at \
             FROM soft_labels WHERE teacher_model_id = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(teacher_id)
        .bind(limit.unwrap_or(100))
        .fetch_all(db.pool())
        .await
        .map_err(|e| {
            crate::domain::error::AppError::DatabaseError(format!(
                "Failed to list soft labels: {e}"
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    } else {
        // List all soft labels
        let rows = sqlx::query_as::<
            _,
            crate::infrastructure::db::training::repositories::SoftLabelEntity,
        >(
            "SELECT soft_label_id, prompt, prompt_hash, teacher_model_id, teacher_output, \
             soft_label_type, temperature, metadata_json, created_at \
             FROM soft_labels \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit.unwrap_or(100))
        .fetch_all(db.pool())
        .await
        .map_err(|e| {
            crate::domain::error::AppError::DatabaseError(format!(
                "Failed to list soft labels: {e}"
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[tauri::command]
pub async fn distill_get_soft_label(
    app: AppHandle,
    _state: State<'_, Arc<AppState>>,
    soft_label_id: String,
) -> Result<SoftLabel> {
    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let sl_repo = SoftLabelRepository::new(&db);

    sl_repo.get(&soft_label_id).await
}

#[tauri::command]
pub async fn distill_delete_soft_label(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    soft_label_id: String,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Deleting soft label: {}", soft_label_id),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let sl_repo = SoftLabelRepository::new(&db);

    sl_repo.delete(&soft_label_id).await
}

#[tauri::command]
pub async fn distill_link_soft_labels_to_run(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    run_id: String,
    soft_label_ids: Vec<String>,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!(
            "Linking {} soft labels to run {}",
            soft_label_ids.len(),
            run_id
        ),
    );

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let sl_repo = SoftLabelRepository::new(&db);

    for soft_label_id in soft_label_ids {
        sl_repo.link_to_run(&run_id, &soft_label_id).await?;
    }

    Ok(())
}
