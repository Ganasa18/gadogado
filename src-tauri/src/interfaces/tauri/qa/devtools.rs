use std::sync::Arc;

use tauri::{Manager, State};

use crate::domain::error::{AppError, Result};
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;

#[tauri::command]
pub async fn qa_open_devtools(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<()> {
    add_log(&state.logs, "INFO", "QA", "QA open devtools requested");
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(any(debug_assertions, feature = "devtools"))]
        {
            window.open_devtools();
            add_log(&state.logs, "INFO", "QA", "QA devtools opened");
        }
        #[cfg(not(any(debug_assertions, feature = "devtools")))]
        {
            add_log(
                &state.logs,
                "WARN",
                "QA",
                "QA devtools unavailable (devtools feature disabled)",
            );
        }
        Ok(())
    } else {
        add_log(
            &state.logs,
            "ERROR",
            "QA",
            "QA devtools failed: main window not found",
        );
        Err(AppError::NotFound("Main window not found.".to_string()))
    }
}
