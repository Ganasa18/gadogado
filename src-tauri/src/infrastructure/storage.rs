use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn resolve_app_data_dir(app_handle: &AppHandle) -> std::io::Result<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    ensure_dir(&app_data_dir)?;
    Ok(app_data_dir)
}

pub fn ensure_qa_sessions_root(app_data_dir: &Path) -> std::io::Result<PathBuf> {
    let qa_sessions_dir = app_data_dir.join("qa_sessions");
    ensure_dir(&qa_sessions_dir)?;
    Ok(qa_sessions_dir)
}

pub fn ensure_session_dir(qa_sessions_dir: &Path, session_id: &str) -> std::io::Result<PathBuf> {
    let session_dir = qa_sessions_dir.join(session_id);
    ensure_dir(&session_dir)?;
    Ok(session_dir)
}

pub fn ensure_session_screenshots_dir(session_dir: &Path) -> std::io::Result<PathBuf> {
    let screenshots_dir = session_dir.join("screenshots");
    ensure_dir(&screenshots_dir)?;
    Ok(screenshots_dir)
}

fn ensure_dir(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
