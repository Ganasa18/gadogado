//! Soft Labels Commands (Phase 1: Data Preparation)
use crate::domain::error::Result;
use crate::infrastructure::db::training::repositories::{
    Model, ModelRepository, SoftLabel, SoftLabelGenerationInput, SoftLabelGenerationResult,
    SoftLabelInput, SoftLabelRepository, TrainingDb,
};
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

use super::common::training_db_path;

#[tauri::command]
pub async fn distill_generate_soft_labels(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: SoftLabelGenerationInput,
) -> Result<SoftLabelGenerationResult> {
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
            "one_hot".to_string()
        } else {
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
            soft_labels_blob: None,
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
    _state: &Arc<AppState>,
    _teacher_model: &Model,
    prompt: &str,
    _temperature: f64,
) -> Result<String> {
    // TODO: Implement actual API call using state.llm_client
    Ok(format!(
        "[Teacher API Response for: {}]",
        &prompt.chars().take(30).collect::<String>()
    ))
}

/// Helper function to call local teacher model
async fn call_teacher_local(
    _state: &Arc<AppState>,
    _teacher_model: &Model,
    prompt: &str,
    _temperature: f64,
) -> Result<String> {
    // TODO: Implement actual local model call
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
