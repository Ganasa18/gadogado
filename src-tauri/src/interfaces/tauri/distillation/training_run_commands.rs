//! Training Run Commands (Flow C - Run Training)
use crate::domain::error::Result;
use crate::infrastructure::db::training::repositories::{
    RunCorrectionsRepository, RunDatasetsRepository, TrainingDb, TrainingLog, TrainingLogInput,
    TrainingLogRepository, TrainingRun, TrainingRunInput, TrainingRunRepository, TrainingStatus,
};
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use std::sync::Arc;
use tauri::{AppHandle, State};

use super::common::training_db_path;

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
