//! Common utilities and types for distillation commands
use crate::domain::error::Result;
use crate::infrastructure::storage::resolve_app_data_dir;
use std::path::PathBuf;
use tauri::AppHandle;

/// Get the path to the training database
pub fn training_db_path(app: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = resolve_app_data_dir(app)?;
    Ok(app_data_dir.join("training.db"))
}
