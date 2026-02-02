//! Model Commands - Model registration and base model management
use crate::domain::error::Result;
use crate::infrastructure::artifact_store::TrainingArtifactLayout;
use crate::infrastructure::db::training::repositories::{
    Model, ModelInput, ModelRepository, TrainingDb,
};
use crate::infrastructure::storage::resolve_app_data_dir;
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{path::BaseDirectory, AppHandle, Manager, State};
use uuid::Uuid;

use super::common::training_db_path;

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
    add_log(
        &state.logs,
        "DEBUG",
        "Distillation",
        &format!("Resource resolve result: {:?}", resource_result),
    );

    if let Ok(resource_base) = resource_result {
        add_log(
            &state.logs,
            "DEBUG",
            "Distillation",
            &format!("Resource base path: {}", resource_base.display()),
        );

        if resource_base.exists() {
            match scan_base_models(&resource_base, "resource") {
                Ok(models) => {
                    add_log(
                        &state.logs,
                        "INFO",
                        "Distillation",
                        &format!("Found {} models in resources", models.len()),
                    );
                    entries.extend(models);
                }
                Err(e) => {
                    add_log(
                        &state.logs,
                        "WARN",
                        "Distillation",
                        &format!("Failed to scan resource models: {}", e),
                    );
                }
            }
        } else {
            // In development mode, try to find resources relative to the executable
            #[cfg(debug_assertions)]
            {
                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        let dev_resource_path = exe_dir.join("resources/models/base");
                        if dev_resource_path.exists() {
                            if let Ok(models) = scan_base_models(&dev_resource_path, "resource") {
                                add_log(
                                    &state.logs,
                                    "INFO",
                                    "Distillation",
                                    &format!("Found {} models in dev resources", models.len()),
                                );
                                entries.extend(models);
                            }
                        }
                    }
                }
            }
        }
    }

    if layout.models_base_dir().exists() {
        match scan_base_models(layout.models_base_dir(), "app_data") {
            Ok(models) => {
                add_log(
                    &state.logs,
                    "INFO",
                    "Distillation",
                    &format!("Found {} models in app_data", models.len()),
                );
                entries.extend(models);
            }
            Err(e) => {
                add_log(
                    &state.logs,
                    "WARN",
                    "Distillation",
                    &format!("Failed to scan app_data models: {}", e),
                );
            }
        }
    }

    add_log(
        &state.logs,
        "INFO",
        "Distillation",
        &format!("Total base models found: {}", entries.len()),
    );
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

    // Create placeholder directory with minimal HuggingFace structure
    std::fs::create_dir_all(&existing_path).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to create model directory {}: {e}",
            existing_path.display()
        ))
    })?;

    let config_path = existing_path.join("config.json");
    let config_content = r#"{"architectures":["LlamaForCausalLM"],"model_type":"llama"}"#;
    std::fs::write(&config_path, config_content).map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to write config.json: {e}"))
    })?;

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

// Helper functions

fn scan_base_models(dir: &Path, source: &str) -> Result<Vec<BaseModelEntry>> {
    let mut out = Vec::new();
    collect_models(dir, source, &mut out)?;
    Ok(out)
}

fn collect_models(dir: &Path, source: &str, out: &mut Vec<BaseModelEntry>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        crate::domain::error::AppError::Internal(format!(
            "Failed to read base model dir {}: {e}",
            dir.display()
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            crate::domain::error::AppError::Internal(format!("Failed to read entry: {e}"))
        })?;
        let path = entry.path();
        if path.file_name().and_then(|s| s.to_str()).unwrap_or("").starts_with('.') {
            continue;
        }

        if path.is_dir() {
            let config_path = path.join("config.json");
            if config_path.exists() {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("model")
                    .to_string();
                out.push(BaseModelEntry {
                    name,
                    path: path.to_string_lossy().to_string(),
                    source: source.to_string(),
                    kind: "dir".to_string(),
                    format: "hf".to_string(),
                });
            } else {
                collect_models(&path, source, out)?;
            }
        } else {
            let name = extract_model_name(&path);
            out.push(BaseModelEntry {
                name,
                path: path.to_string_lossy().to_string(),
                source: source.to_string(),
                kind: "file".to_string(),
                format: detect_model_format(&path),
            });
        }
    }
    Ok(())
}

fn extract_model_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| {
            let mut name = s.to_string();
            for suffix in &["_q5_k_m", "_q4_k_m", "_q8_0", "-q5_k_m", "-q4_k_m", "-q8_0"] {
                name = name.strip_suffix(suffix).unwrap_or(&name).to_string();
            }
            name = name.strip_suffix("-instruct").unwrap_or(&name).to_string();
            name = name.strip_suffix(".gguf").unwrap_or(&name).to_string();
            name = name.strip_suffix(".GGUF").unwrap_or(&name).to_string();
            name
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("model-{}", &Uuid::new_v4().to_string()[..8]))
}

fn detect_model_format(path: &Path) -> String {
    if path.is_file() {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("gguf") { return "gguf".to_string(); }
            if ext.eq_ignore_ascii_case("bin") { return "bin".to_string(); }
            if ext.eq_ignore_ascii_case("safetensors") { return "safetensors".to_string(); }
        }
        return "unknown".to_string();
    }
    if path.join("config.json").exists() { return "hf".to_string(); }
    "unknown".to_string()
}

pub fn sanitize_model_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_whitespace() {
            out.push('-');
        }
    }
    if out.is_empty() { "base-model".to_string() } else { out }
}

pub fn unique_path(base: &Path, name: &str, ext: Option<&str>) -> Result<PathBuf> {
    let mut candidate = match ext {
        Some(ext) => base.join(format!("{}.{}", name, ext)),
        None => base.join(name),
    };
    if !candidate.exists() {
        return Ok(candidate);
    }
    let short = &Uuid::new_v4().to_string()[..8];
    candidate = match ext {
        Some(ext) => base.join(format!("{}-{}.{}", name, short, ext)),
        None => base.join(format!("{}-{}", name, short)),
    };
    Ok(candidate)
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to create dir {}: {e}", dst.display()))
    })?;
    for entry in std::fs::read_dir(src).map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to read dir {}: {e}", src.display()))
    })? {
        let entry = entry.map_err(|e| {
            crate::domain::error::AppError::Internal(format!("Failed to read entry: {e}"))
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
