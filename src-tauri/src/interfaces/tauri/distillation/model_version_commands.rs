//! Model Version Commands (Flow E - Promote Version + Rollback)
use crate::domain::error::Result;
use crate::infrastructure::artifact_store::{backup_training_db, BackupConfig};
use crate::infrastructure::db::training::repositories::{
    ActiveModelRepository, EvaluationMetricsRepository, ModelVersion, ModelVersionInput,
    ModelVersionRepository, TrainingDb,
};
use crate::infrastructure::storage::resolve_app_data_dir;
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tracing::error;

use super::common::training_db_path;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    pub previous_version_id: Option<String>,
    pub rolled_back_to: ModelVersion,
    pub backup_created: bool,
}

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
                    passed: true,
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

    let active_repo = ActiveModelRepository::new(&db);
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
