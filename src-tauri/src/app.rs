use std::sync::Arc;

use tauri::Manager;

pub fn run() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| crate::infrastructure::bootstrap::setup(app))
        .invoke_handler(crate::tauri_invoke_handler!())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() != "main" {
                    return;
                }

                let app_handle = window.app_handle().clone();

                if let Some(state) = app_handle.try_state::<Arc<crate::interfaces::tauri::AppState>>()
                {
                    tauri::async_runtime::block_on(async {
                        crate::interfaces::tauri::cleanup_child_processes(&state).await;
                    });
                }

                app_handle.exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
