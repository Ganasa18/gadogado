//! Python Orchestrator (Rust -> Python runner)
use crate::domain::error::Result;
use crate::infrastructure::artifact_store::TrainingArtifactLayout;
use crate::infrastructure::db::training::repositories::{
    ModelVersionInput, ModelVersionRepository, RunArtifact, RunArtifactInput, RunArtifactsRepository,
    TrainingDb, TrainingLogInput, TrainingLogRepository, TrainingRunRepository, TrainingStatus,
};
use crate::infrastructure::storage::resolve_app_data_dir;
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::{AppState, DistillTrainerHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager, State};
use tokio::fs::OpenOptions as TokioOpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;

use super::common::training_db_path;

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

struct DistillTrainerLaunchGuard {
    state: Arc<AppState>,
    run_id: String,
    active: bool,
}

impl DistillTrainerLaunchGuard {
    fn reserve(state: Arc<AppState>, run_id: &str) -> Result<Self> {
        {
            let trainers = state.distill_trainers.lock().unwrap();
            if trainers.contains_key(run_id) {
                return Err(crate::domain::error::AppError::ValidationError(format!(
                    "Trainer already running for run_id={}", run_id
                )));
            }
        }
        {
            let mut launches = state.distill_trainer_launches.lock().unwrap();
            if launches.contains(run_id) {
                return Err(crate::domain::error::AppError::ValidationError(format!(
                    "Trainer already starting for run_id={}", run_id
                )));
            }
            launches.insert(run_id.to_string());
        }
        Ok(Self { state, run_id: run_id.to_string(), active: true })
    }

    fn insert_handle(&mut self, handle: DistillTrainerHandle) -> Result<()> {
        let mut trainers = self.state.distill_trainers.lock().unwrap();
        let mut launches = self.state.distill_trainer_launches.lock().unwrap();
        if trainers.contains_key(&self.run_id) {
            return Err(crate::domain::error::AppError::ValidationError(format!(
                "Trainer already running for run_id={}", self.run_id
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
        if !self.active { return; }
        let mut launches = self.state.distill_trainer_launches.lock().unwrap();
        launches.remove(&self.run_id);
    }
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

    let mut launch_guard = DistillTrainerLaunchGuard::reserve(state.inner().clone(), &run_id)?;

    add_log(&state.logs, "INFO", "Distillation", &format!("Starting python trainer for run {}", run_id));

    let app_data_dir = resolve_app_data_dir(&app)?;
    let layout = TrainingArtifactLayout::new(&app_data_dir);
    layout.ensure()?;

    let run_dir = if config.run_dir.trim().is_empty() {
        layout.run_dir(&run_id)
    } else {
        PathBuf::from(&config.run_dir)
    };
    std::fs::create_dir_all(&run_dir).map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to create run_dir {}: {e}", run_dir.display()))
    })?;

    let cancel_flag_path = run_dir.join("cancel.flag");
    let _ = std::fs::remove_file(&cancel_flag_path);

    let script_path = resolve_train_script_path(&app)?;

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let run_repo = TrainingRunRepository::new(&db);
    let _run = run_repo.get(&run_id).await?;

    let config_path = run_dir.join("trainer_config.json");
    let cfg_json = serde_json::json!({
        "run_id": run_id.clone(),
        "run_dir": run_dir.to_string_lossy(),
        "mode": config.mode.clone().unwrap_or_else(|| "fine_tune".to_string()),
        "seed": config.seed,
        "steps": config.steps.unwrap_or(100),
        "emit_every": config.emit_every.unwrap_or(1),
        "training_db_path": db_path.to_string_lossy(),
        "dataset_source": "db",
        "hyperparams": config.hyperparams.clone(),
    });
    if let Err(e) = std::fs::write(&config_path, serde_json::to_vec_pretty(&cfg_json).unwrap()) {
        let err = crate::domain::error::AppError::Internal(format!(
            "Failed to write trainer_config.json {}: {e}", config_path.display()
        ));
        let _ = run_repo.set_status(&run_id, TrainingStatus::Failed, Some(chrono::Utc::now().to_rfc3339()), Some(err.to_string())).await;
        return Err(err);
    }

    let stdout_log_path = run_dir.join("trainer_stdout.log");
    let stderr_log_path = run_dir.join("trainer_stderr.log");
    let metrics_log_path = run_dir.join("trainer_metrics.jsonl");
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&stdout_log_path);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&stderr_log_path);

    // Record baseline artifacts
    let artifacts_repo = RunArtifactsRepository::new(&db);
    record_initial_artifacts(&db, &run_id, &config_path, &stdout_log_path, &stderr_log_path).await;

    // Spawn Python process
    let mut cmd = TokioCommand::new("python");
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
            let err = crate::domain::error::AppError::Internal(format!("Failed to spawn python: {e}"));
            let _ = run_repo.set_status(&run_id, TrainingStatus::Failed, Some(chrono::Utc::now().to_rfc3339()), Some(err.to_string())).await;
            return Err(err);
        }
    };

    let stdout = child.stdout.take().ok_or_else(|| {
        crate::domain::error::AppError::Internal("Trainer stdout unavailable".to_string())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        crate::domain::error::AppError::Internal("Trainer stderr unavailable".to_string())
    })?;

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

    if let Err(e) = run_repo.set_status(&run_id, TrainingStatus::Running, None, None).await {
        add_log(&state.logs, "ERROR", "Distillation", &format!("Failed to mark run as running: {e}"));
    }

    let last_error: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));

    // Spawn handlers
    spawn_train_stdout_handler(
        app.clone(), state.logs.clone(), db_path.clone(), run_id.clone(),
        run_dir.clone(), stdout_log_path, metrics_log_path, stdout, last_error.clone()
    );
    spawn_train_stderr_handler(
        app.clone(), state.logs.clone(), stderr_log_path, stderr, last_error.clone()
    );
    spawn_train_exit_monitor(
        app.clone(), state.inner().clone(), db_path, run_id.clone(),
        run_dir.clone(), child.clone(), last_error
    );

    let token = format!("{}:{}", run_id, Uuid::new_v4());
    add_log(&state.logs, "INFO", "Distillation", &format!("Python trainer spawned (token={})", token));

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
        return Err(crate::domain::error::AppError::ValidationError("run_id is required".to_string()));
    }

    add_log(&state.logs, "INFO", "Distillation", &format!("Cancelling python trainer for run {}", run_id));

    let (child, run_dir) = {
        let guard = state.distill_trainers.lock().unwrap();
        let handle = guard.get(&run_id).ok_or_else(|| {
            crate::domain::error::AppError::ValidationError(format!("No active trainer for run_id={}", run_id))
        })?;
        (handle.child.clone(), handle.run_dir.clone())
    };

    let cancel_flag_path = run_dir.join("cancel.flag");
    tokio::fs::write(&cancel_flag_path, b"cancel\n").await.map_err(|e| {
        crate::domain::error::AppError::Internal(format!("Failed to write cancel flag {}: {e}", cancel_flag_path.display()))
    })?;

    let db_path = training_db_path(&app)?;
    let db = TrainingDb::connect(&db_path).await?;
    let run_repo = TrainingRunRepository::new(&db);
    let _ = run_repo.set_status(&run_id, TrainingStatus::Cancelled, Some(chrono::Utc::now().to_rfc3339()), None).await;

    // Force-kill after grace period
    tauri::async_runtime::spawn(async move {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let exited = {
                let mut guard = child.lock().await;
                matches!(guard.try_wait(), Ok(Some(_)) | Err(_))
            };
            if exited { break; }
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

fn resolve_train_script_path(app: &AppHandle) -> Result<PathBuf> {
    let script_path: PathBuf = app
        .path()
        .resolve("resources/scripts/distill-train.py", BaseDirectory::Resource)
        .or_else(|_| {
            let cwd = std::env::current_dir().map_err(|e| {
                crate::domain::error::AppError::Internal(format!("Failed to resolve cwd: {e}"))
            })?;
            for path in &[
                cwd.join("src-tauri/resources/scripts/distill-train.py"),
                cwd.join("target/debug/resources/scripts/distill-train.py"),
            ] {
                if path.exists() {
                    return Ok::<PathBuf, crate::domain::error::AppError>(path.clone());
                }
            }
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let exe_relative = exe_dir.join("resources/scripts/distill-train.py");
                    if exe_relative.exists() {
                        return Ok::<PathBuf, crate::domain::error::AppError>(exe_relative);
                    }
                }
            }
            Err(crate::domain::error::AppError::NotFound("Python trainer script not found".to_string()))
        })?;

    let path_str = script_path.to_string_lossy();
    if path_str.starts_with(r"\\?\") {
        Ok(PathBuf::from(&path_str[4..]))
    } else {
        Ok(script_path)
    }
}

async fn record_initial_artifacts(
    db: &TrainingDb,
    run_id: &str,
    config_path: &PathBuf,
    stdout_log_path: &PathBuf,
    stderr_log_path: &PathBuf,
) {
    let repo = RunArtifactsRepository::new(db);
    for (kind, path) in [
        ("config", config_path),
        ("log", stdout_log_path),
        ("log", stderr_log_path),
    ] {
        let _ = repo.insert(&RunArtifactInput {
            artifact_id: Uuid::new_v4().to_string(),
            run_id: run_id.to_string(),
            kind: kind.to_string(),
            path: path.to_string_lossy().to_string(),
            hash: None,
            size_bytes: None,
        }).await;
    }
}

fn spawn_train_stdout_handler(
    app: AppHandle,
    logs: Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    db_path: PathBuf,
    run_id: String,
    run_dir: PathBuf,
    stdout_log_path: PathBuf,
    metrics_log_path: PathBuf,
    stdout: tokio::process::ChildStdout,
    last_error: Arc<std::sync::Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let db = match TrainingDb::connect(&db_path).await { Ok(db) => db, Err(_) => return };
        let log_repo = TrainingLogRepository::new(&db);
        let run_repo = TrainingRunRepository::new(&db);
        let artifacts_repo = RunArtifactsRepository::new(&db);

        let mut stdout_log = match TokioOpenOptions::new().create(true).append(true).open(&stdout_log_path).await {
            Ok(f) => f, Err(_) => return
        };
        let mut metrics_log = TokioOpenOptions::new().create(true).append(true).open(&metrics_log_path).await.ok();

        let mut lines = TokioBufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stdout_log.write_all(line.as_bytes()).await;
            let _ = stdout_log.write_all(b"\n").await;

            let Ok(msg) = serde_json::from_str::<DistillPythonMessage>(&line) else { continue };

            let _ = app.emit("distill-train-stream", msg.clone());

            match msg.kind.as_str() {
                "progress" => {
                    let payload = &msg.payload;
                    let epoch = payload.get("epoch").and_then(|v| v.as_i64()).unwrap_or(0);
                    let step = payload.get("step").and_then(|v| v.as_i64()).unwrap_or(0);
                    let loss = payload.get("loss").and_then(|v| v.as_f64());
                    let lr = payload.get("lr").and_then(|v| v.as_f64());
                    let temperature = payload.get("temperature").and_then(|v| v.as_f64());
                    let resources = payload.get("resources").and_then(|v| v.as_object());
                    let cpu_util = resources.and_then(|o| o.get("cpu_percent")).and_then(|v| v.as_f64());
                    let ram_usage_mb = resources.and_then(|o| o.get("ram_rss_bytes")).and_then(|v| v.as_i64()).map(|b| b / (1024 * 1024));
                    let gpu_util = resources.and_then(|o| o.get("gpu_util_percent")).and_then(|v| v.as_f64());

                    let _ = log_repo.insert(&TrainingLogInput {
                        run_id: run_id.clone(), epoch, step, loss, lr, temperature, cpu_util, ram_usage_mb, gpu_util
                    }).await;
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
                        "config" | "log" | "checkpoint" | "adapter" | "merged_model" | "gguf" => Some(kind),
                        "model" => Some("merged_model"),
                        "result" => Some("log"),
                        _ => None,
                    };
                    if let (Some(k), false) = (db_kind, path.trim().is_empty()) {
                        let _ = artifacts_repo.insert(&RunArtifactInput {
                            artifact_id: Uuid::new_v4().to_string(),
                            run_id: run_id.clone(),
                            kind: k.to_string(),
                            path: path.to_string(),
                            hash: None,
                            size_bytes: None,
                        }).await;
                    }
                }
                "status" => {
                    let payload = &msg.payload;
                    let level = payload.get("level").and_then(|v| v.as_str()).unwrap_or("");
                    let message = payload.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if level.eq_ignore_ascii_case("error") {
                        { let mut guard = last_error.lock().unwrap(); *guard = Some(message.clone()); }
                        if let Some(trace) = payload.get("trace").and_then(|v| v.as_str()) {
                            let trace_path = run_dir.join("trainer_error.trace");
                            let _ = tokio::fs::write(trace_path, trace).await;
                        }
                        let _ = run_repo.set_status(&run_id, TrainingStatus::Failed, None, Some(message)).await;
                    }
                }
                _ => {}
            }
        }
    });
}

fn spawn_train_stderr_handler(
    app: AppHandle,
    _logs: Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    stderr_log_path: PathBuf,
    stderr: tokio::process::ChildStderr,
    last_error: Arc<std::sync::Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut stderr_log = match TokioOpenOptions::new().create(true).append(true).open(&stderr_log_path).await {
            Ok(f) => f, Err(_) => return
        };

        let mut lines = TokioBufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_log.write_all(line.as_bytes()).await;
            let _ = stderr_log.write_all(b"\n").await;

            if !line.trim().is_empty() {
                let mut guard = last_error.lock().unwrap();
                if guard.is_none() {
                    let trimmed = if line.len() > 400 { format!("{}...", &line[..400]) } else { line.clone() };
                    *guard = Some(trimmed);
                }
                drop(guard);
                let msg = serde_json::json!({"kind": "stderr", "payload": {"message": line}});
                let _ = app.emit("distill-train-stream", msg);
            }
        }
    });
}

fn spawn_train_exit_monitor(
    app: AppHandle,
    state: Arc<AppState>,
    db_path: PathBuf,
    run_id: String,
    run_dir: PathBuf,
    child: Arc<AsyncMutex<tokio::process::Child>>,
    last_error: Arc<std::sync::Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let db = match TrainingDb::connect(&db_path).await { Ok(db) => db, Err(_) => return };
        let run_repo = TrainingRunRepository::new(&db);

        loop {
            let exited = {
                let mut guard = child.lock().await;
                match guard.try_wait() {
                    Ok(Some(status)) => Some(Ok(status)),
                    Ok(None) => None,
                    Err(err) => Some(Err(err)),
                }
            };

            let outcome = match exited {
                None => { sleep(Duration::from_millis(250)).await; continue; }
                Some(v) => v,
            };

            let cancelled = run_dir.join("cancel.flag").exists();
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
                        let guard = last_error.lock().unwrap();
                        guard.clone().or_else(|| Some(format!("Trainer exited: {}", status)))
                    } else { None };

                    let is_completed = matches!(final_status, TrainingStatus::Completed);
                    let _ = run_repo.set_status(&run_id, final_status, end_time, failure_reason).await;

                    if is_completed {
                        auto_create_model_version(&db, &run_id, &state.logs).await;
                    }
                }
                Err(err) => {
                    let failure = { let guard = last_error.lock().unwrap(); guard.clone().unwrap_or_else(|| format!("Trainer wait failed: {}", err)) };
                    let _ = run_repo.set_status(&run_id, TrainingStatus::Failed, end_time, Some(failure)).await;
                }
            }

            { let mut guard = state.distill_trainers.lock().unwrap(); guard.remove(&run_id); }

            let _ = app.emit("distill-train-stream", DistillPythonMessage {
                kind: "status".to_string(),
                payload: serde_json::json!({"level": "info", "message": "trainer exited", "run_id": run_id, "cancelled": cancelled}),
            });
            break;
        }
    });
}

async fn auto_create_model_version(
    db: &TrainingDb,
    run_id: &str,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) {
    let run_repo = TrainingRunRepository::new(db);
    let version_repo = ModelVersionRepository::new(db);
    let artifacts_repo = RunArtifactsRepository::new(db);
    if let Ok(existing) = version_repo.find_by_run_id(run_id).await {
        if existing.is_some() { return; }
    }

    let Ok(run) = run_repo.get(run_id).await else { return };
    let Ok(artifacts) = artifacts_repo.list_for_run(run_id).await else { return };

    let preferred = ["merged_model", "adapter", "gguf"];
    let mut picked: Option<&RunArtifact> = None;
    for kind in preferred {
        if let Some(found) = artifacts.iter().find(|a| a.kind == kind) {
            picked = Some(found);
            break;
        }
    }

    if let Some(artifact) = picked {
        let version_id = Uuid::new_v4().to_string();
        let input = ModelVersionInput {
            version_id: version_id.clone(),
            model_id: run.student_model_id.clone(),
            run_id: Some(run_id.to_string()),
            parent_version_id: run.base_version_id.clone(),
            artifact_path: artifact.path.clone(),
            artifact_hash: artifact.hash.clone(),
            artifact_size_bytes: artifact.size_bytes,
            notes: Some(format!("Auto-created from run {}", run_id)),
        };
        if let Err(e) = version_repo.insert(&input).await {
            add_log(logs, "ERROR", "Distillation", &format!("Failed to auto-create model version: {e}"));
        } else {
            add_log(logs, "INFO", "Distillation", &format!("Auto-created model version {} for run {}", version_id, run_id));
        }
    }
}
