//! Artifact and Backup Commands
//! - Evaluation Commands (Flow D - Evaluate + Compare)
//! - Run Artifacts Commands
//! - Backup Commands
//! - Artifact Layout Commands
use crate::domain::error::Result;
use crate::infrastructure::artifact_store::{
    backup_training_db, cleanup_old_backups, list_backups, restore_from_backup, BackupConfig,
    BackupInfo, TrainingArtifactLayout,
};
use crate::infrastructure::db::training::repositories::{
    EvaluationMetric, EvaluationMetricInput, EvaluationMetricsRepository, RunArtifact,
    RunArtifactInput, RunArtifactsRepository, TrainingDb,
};
use crate::infrastructure::storage::resolve_app_data_dir;
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

use super::common::training_db_path;

// ============================================================================
// Evaluation Commands
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
