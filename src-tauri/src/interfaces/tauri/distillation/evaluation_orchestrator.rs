//! Evaluation Orchestrator (Rust -> Python evaluator)
use crate::domain::error::Result;
use crate::infrastructure::artifact_store::TrainingArtifactLayout;
use crate::infrastructure::db::training::repositories::{
    EvaluationMetricInput, EvaluationMetricsRepository, TrainingDb,
};
use crate::infrastructure::storage::resolve_app_data_dir;
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as AsyncMutex;
use tokio::fs::OpenOptions as TokioOpenOptions;
use uuid::Uuid;

use super::common::training_db_path;
use super::python_orchestrator::DistillPythonMessage;

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

    let script_path = resolve_eval_script_path(&app)?;

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

    // Touch log files
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&stdout_log_path);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&stderr_log_path);

    // Spawn python evaluator
    let mut cmd = TokioCommand::new("python");
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

    // Spawn stdout handler
    spawn_eval_stdout_handler(
        app.clone(),
        db_path.clone(),
        version_id.clone(),
        dataset_id.clone(),
        stdout_log_path.clone(),
        metrics_log_path.clone(),
        stdout,
        last_error.clone(),
    );

    // Spawn stderr handler
    spawn_eval_stderr_handler(stderr_log_path.clone(), stderr, last_error.clone());

    // Spawn exit monitor
    spawn_eval_exit_monitor(app.clone(), eval_id.clone(), child.clone(), last_error.clone());

    Ok(eval_id)
}

fn resolve_eval_script_path(app: &AppHandle) -> Result<PathBuf> {
    let script_path: PathBuf = app
        .path()
        .resolve("resources/scripts/distill-eval.py", tauri::path::BaseDirectory::Resource)
        .or_else(|_| {
            let cwd = std::env::current_dir().map_err(|e| {
                crate::domain::error::AppError::Internal(format!("Failed to resolve cwd: {e}"))
            })?;

            for path in &[
                cwd.join("src-tauri/resources/scripts/distill-eval.py"),
                cwd.join("target/debug/resources/scripts/distill-eval.py"),
            ] {
                if path.exists() {
                    return Ok::<PathBuf, crate::domain::error::AppError>(path.clone());
                }
            }

            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let exe_relative = exe_dir.join("resources/scripts/distill-eval.py");
                    if exe_relative.exists() {
                        return Ok::<PathBuf, crate::domain::error::AppError>(exe_relative);
                    }
                }
            }

            Err(crate::domain::error::AppError::NotFound(
                "Python evaluator script not found".to_string(),
            ))
        })?;

    // Strip Windows extended path prefix
    let path_str = script_path.to_string_lossy();
    if path_str.starts_with(r"\\?\") {
        Ok(PathBuf::from(&path_str[4..]))
    } else {
        Ok(script_path)
    }
}

fn spawn_eval_stdout_handler(
    app: AppHandle,
    db_path: PathBuf,
    version_id: String,
    dataset_id: String,
    stdout_log_path: PathBuf,
    metrics_log_path: PathBuf,
    stdout: tokio::process::ChildStdout,
    last_error: Arc<std::sync::Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let db = match TrainingDb::connect(&db_path).await {
            Ok(db) => db,
            Err(_) => return,
        };
        let metrics_repo = EvaluationMetricsRepository::new(&db);

        let mut stdout_log = match TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&stdout_log_path)
            .await
        {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut metrics_log = TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&metrics_log_path)
            .await
            .ok();

        let mut lines = TokioBufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stdout_log.write_all(line.as_bytes()).await;
            let _ = stdout_log.write_all(b"\n").await;

            let Ok(msg) = serde_json::from_str::<DistillPythonMessage>(&line) else {
                continue;
            };

            let _ = app.emit("distill-eval-stream", msg.clone());

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
                                version_id: version_id.clone(),
                                dataset_id: dataset_id.clone(),
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
                        let mut guard = last_error.lock().unwrap();
                        *guard = Some(message);
                    }
                }
                _ => {}
            }
        }
    });
}

fn spawn_eval_stderr_handler(
    stderr_log_path: PathBuf,
    stderr: tokio::process::ChildStderr,
    last_error: Arc<std::sync::Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut stderr_log = match TokioOpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_log_path)
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
                let mut guard = last_error.lock().unwrap();
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
}

fn spawn_eval_exit_monitor(
    app: AppHandle,
    eval_id: String,
    child: Arc<AsyncMutex<tokio::process::Child>>,
    last_error: Arc<std::sync::Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let status = {
            let mut guard = child.lock().await;
            guard.wait().await
        };

        let failed = status.map(|s| !s.success()).unwrap_or(true);
        if failed {
            let guard = last_error.lock().unwrap();
            let msg = guard
                .clone()
                .unwrap_or_else(|| "Evaluator exited with failure".to_string());
            let _ = app.emit(
                "distill-eval-stream",
                DistillPythonMessage {
                    kind: "status".to_string(),
                    payload: serde_json::json!({
                        "level": "error",
                        "message": msg,
                        "eval_id": eval_id,
                    }),
                },
            );
        } else {
            let _ = app.emit(
                "distill-eval-stream",
                DistillPythonMessage {
                    kind: "status".to_string(),
                    payload: serde_json::json!({
                        "level": "info",
                        "message": "evaluation completed",
                        "eval_id": eval_id,
                    }),
                },
            );
        }
    });
}
