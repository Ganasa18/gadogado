mod application;
mod domain;
mod infrastructure;
mod interfaces;

use crate::application::use_cases::qa_ai::QaAiUseCase;
use crate::application::{
    EnhanceUseCase, QaApiCallUseCase, QaEventUseCase, QaRunUseCase, QaSessionUseCase,
    TranslateUseCase, TypeGenUseCase,
};
use crate::infrastructure::config::ConfigService;
use crate::infrastructure::db::qa::init_qa_db;
use crate::infrastructure::db::qa_api_calls::QaApiCallRepository;
use crate::infrastructure::db::qa_checkpoints::QaCheckpointRepository;
use crate::infrastructure::db::qa_events::QaEventRepository;
use crate::infrastructure::db::qa_runs::QaRunRepository;
use crate::infrastructure::db::qa_sessions::QaRepository;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::llm_clients::RouterClient;
use crate::infrastructure::storage::{
    ensure_qa_sessions_root, ensure_session_dir, resolve_app_data_dir,
};
use crate::interfaces::tauri::{
    delete_api_key, enhance_prompt, get_api_key, get_llm_models, get_translation_history,
    qa_append_run_stream_event, qa_capture_native_screenshot, qa_capture_screenshot,
    qa_create_checkpoint, qa_delete_events, qa_delete_session, qa_end_run, qa_end_session,
    qa_execute_api_request, qa_explore_session, qa_generate_checkpoint_summary,
    qa_generate_test_cases, qa_get_session, qa_list_checkpoint_summaries, qa_list_checkpoints,
    qa_list_events, qa_list_events_page, qa_list_llm_runs, qa_list_run_stream_events,
    qa_list_screenshots, qa_list_sessions, qa_list_test_cases, qa_open_devtools, qa_record_event,
    qa_replay_browser, qa_start_browser_recorder, qa_start_run, qa_start_session,
    qa_stop_browser_recorder, save_api_key, sync_config, sync_languages, sync_shortcuts,
    translate_prompt, AppState,
};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_clipboard_manager::{Clipboard, ClipboardExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::error;
use uuid::Uuid;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            let app_data_dir = resolve_app_data_dir(&app_handle).map_err(|err| {
                error!(error = %err, "Failed to resolve app data dir");
                err
            })?;
            let qa_sessions_dir = ensure_qa_sessions_root(&app_data_dir).map_err(|err| {
                error!(
                    error = %err,
                    qa_sessions_dir = %app_data_dir.join("qa_sessions").display(),
                    "Failed to create qa_sessions dir"
                );
                err
            })?;
            let bootstrap_session_id = Uuid::new_v4().to_string();
            let _bootstrap_session_dir =
                ensure_session_dir(&qa_sessions_dir, &bootstrap_session_id).map_err(|err| {
                    error!(
                        error = %err,
                        session_id = %bootstrap_session_id,
                        session_dir = %qa_sessions_dir.join(&bootstrap_session_id).display(),
                        "Failed to create QA session dir"
                    );
                    err
                })?;

            let qa_db_path = app_data_dir.join("qa_recorder.db");
            println!("Initializing QA database at: {}", qa_db_path.display());

            let db_path = app_data_dir.join("promptbridge.db");
            let db_path_str = db_path.to_string_lossy().replace("\\", "/");
            let db_url = format!("sqlite://{}", db_path_str);

            println!("Initializing database at: {}", db_url);

            tauri::async_runtime::block_on(async move {
                init_qa_db(&qa_db_path)
                    .await
                    .expect("Failed to initialize QA database");
                println!("Initialized QA database at: {}", qa_db_path.display());
                match std::fs::metadata(&qa_db_path) {
                    Ok(meta) => println!(
                        "QA database file created: {} ({} bytes)",
                        qa_db_path.display(),
                        meta.len()
                    ),
                    Err(err) => println!(
                        "QA database file missing after init: {} ({})",
                        qa_db_path.display(),
                        err
                    ),
                }
                println!("QA database ready, proceeding to app database init");
                let qa_repo = QaRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA database");
                let qa_repo_arc = Arc::new(qa_repo);
                let qa_event_repo = QaEventRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA events database");
                let qa_event_repo_arc = Arc::new(qa_event_repo);
                let qa_checkpoint_repo = QaCheckpointRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA checkpoints database");
                let qa_checkpoint_repo_arc = Arc::new(qa_checkpoint_repo);
                let qa_run_repo = QaRunRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA runs database");
                let qa_run_repo_arc = Arc::new(qa_run_repo);
                let qa_api_call_repo = QaApiCallRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA API calls database");
                let qa_api_call_repo_arc = Arc::new(qa_api_call_repo);
                let repo = SqliteRepository::init(&db_url)
                    .await
                    .expect("Failed to initialize database");
                let repo_arc = Arc::new(repo);

                let logs = Arc::new(Mutex::new(Vec::new()));
                let logs_for_server = logs.clone();
                let logs_for_setup = logs.clone();

                let llm_client: Arc<dyn LLMClient + Send + Sync> = Arc::new(RouterClient::new());

                let translate_use_case =
                    TranslateUseCase::new(llm_client.clone(), repo_arc.clone());
                let enhance_use_case = EnhanceUseCase::new(llm_client.clone(), repo_arc.clone());

                let typegen_use_case = TypeGenUseCase::new(llm_client.clone());
                let qa_session_use_case =
                    QaSessionUseCase::new(qa_repo_arc.clone(), qa_sessions_dir.clone());
                let qa_event_use_case = QaEventUseCase::new(qa_event_repo_arc.clone());
                let qa_run_use_case = QaRunUseCase::new(qa_run_repo_arc.clone());
                let qa_api_call_use_case = QaApiCallUseCase::new(qa_api_call_repo_arc.clone());
                let qa_ai_use_case = QaAiUseCase::new(
                    qa_repo_arc.clone(),
                    qa_event_repo_arc.clone(),
                    qa_checkpoint_repo_arc.clone(),
                    llm_client.clone(),
                );

                let state = AppState {
                    translate_use_case,
                    enhance_use_case,
                    typegen_use_case,
                    qa_session_use_case,
                    qa_event_use_case,
                    qa_ai_use_case,
                    qa_run_use_case,
                    qa_api_call_use_case,
                    qa_session_id: Mutex::new(None),
                    qa_recorder: Mutex::new(None),
                    repository: repo_arc,
                    config_service: ConfigService::new(),
                    llm_client: llm_client.clone(),
                    last_config: Mutex::new(crate::domain::llm_config::LLMConfig::default()),
                    preferred_source: Mutex::new("Auto Detect".to_string()),
                    preferred_target: Mutex::new("English".to_string()),
                    logs: logs.clone(),
                };
                let state_arc = Arc::new(state);

                app_handle.manage(state_arc.clone());

                // Start Actix server
                let state_for_server = state_arc.clone();
                let server =
                    crate::interfaces::http::start_server(state_for_server, logs_for_server)
                        .expect("Failed to start Actix server");

                tokio::spawn(server);

                crate::interfaces::http::add_log(
                    &logs_for_setup,
                    "INFO",
                    "System",
                    "Backend initialized and HTTP server started on :3001",
                );

                if let Err(e) = register_shortcuts(
                    &app_handle,
                    true,
                    "Ctrl + Alt + T",
                    "Ctrl + Alt + E",
                    "Ctrl + Alt + P",
                    "Ctrl + Alt + R",
                ) {
                    eprintln!("Failed to register default shortcuts: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            translate_prompt,
            enhance_prompt,
            get_translation_history,
            save_api_key,
            get_api_key,
            delete_api_key,
            get_llm_models,
            sync_config,
            sync_languages,
            sync_shortcuts,
            qa_start_session,
            qa_end_session,
            qa_start_run,
            qa_end_run,
            qa_start_browser_recorder,
            qa_stop_browser_recorder,
            qa_append_run_stream_event,
            qa_list_run_stream_events,
            qa_execute_api_request,
            qa_replay_browser,
            qa_record_event,
            qa_open_devtools,
            qa_list_sessions,
            qa_list_events,
            qa_list_screenshots,
            qa_capture_screenshot,
            qa_capture_native_screenshot,
            qa_list_events_page,
            qa_delete_events,
            qa_delete_session,
            qa_get_session,
            qa_create_checkpoint,
            qa_list_checkpoints,
            qa_generate_checkpoint_summary,
            qa_generate_test_cases,
            qa_list_checkpoint_summaries,
            qa_list_test_cases,
            qa_list_llm_runs,
            qa_explore_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use enigo::{Enigo, Key, KeyboardControllable};
use std::time::Duration;

fn log_shortcut(state: &tauri::State<'_, Arc<AppState>>, level: &str, message: &str) {
    crate::interfaces::http::add_log(&state.logs, level, "Shortcut", message);
}

fn emit_shortcut_event(app: &tauri::AppHandle, event: &str, payload: &str) {
    if let Err(e) = app.emit(event, payload) {
        eprintln!("Failed to emit event: {}", e);
    }
}

fn parse_shortcut(input: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = input
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();

    if parts.is_empty() {
        return Err("Shortcut is empty.".to_string());
    }

    let key_part = parts[parts.len() - 1];
    let mut modifiers = Modifiers::empty();

    for modifier in &parts[..parts.len() - 1] {
        match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "cmd" | "meta" | "win" | "super" => modifiers |= Modifiers::META,
            other => {
                return Err(format!("Unknown modifier: {}", other));
            }
        }
    }

    let code = parse_code(key_part)?;
    let modifiers = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Ok(Shortcut::new(modifiers, code))
}

fn parse_code(key: &str) -> Result<Code, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Shortcut key is empty.".to_string());
    }

    let normalized = match key {
        "Esc" => "Escape".to_string(),
        "Space" => "Space".to_string(),
        "Enter" => "Enter".to_string(),
        "Tab" => "Tab".to_string(),
        "Backspace" => "Backspace".to_string(),
        "Delete" => "Delete".to_string(),
        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => key.to_string(),
        "PageUp" | "PageDown" | "Home" | "End" | "Insert" => key.to_string(),
        "Minus" | "Equal" | "Comma" | "Period" | "Slash" | "Semicolon" | "Quote"
        | "BracketLeft" | "BracketRight" | "Backslash" | "Backquote" => key.to_string(),
        _ if key.len() == 1 => {
            let ch = key.chars().next().unwrap();
            if ch.is_ascii_alphabetic() {
                format!("Key{}", ch.to_ascii_uppercase())
            } else if ch.is_ascii_digit() {
                format!("Digit{}", ch)
            } else {
                match ch {
                    '-' => "Minus".to_string(),
                    '=' => "Equal".to_string(),
                    ',' => "Comma".to_string(),
                    '.' => "Period".to_string(),
                    '/' => "Slash".to_string(),
                    ';' => "Semicolon".to_string(),
                    '\'' => "Quote".to_string(),
                    '[' => "BracketLeft".to_string(),
                    ']' => "BracketRight".to_string(),
                    '\\' => "Backslash".to_string(),
                    '`' => "Backquote".to_string(),
                    _ => return Err(format!("Unsupported key: {}", key)),
                }
            }
        }
        _ if key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit()) => {
            key.to_string()
        }
        _ => key.to_string(),
    };

    Code::from_str(&normalized).map_err(|_| format!("Unsupported key: {}", key))
}

pub(crate) fn register_shortcuts(
    app: &tauri::AppHandle,
    enabled: bool,
    translate: &str,
    enhance: &str,
    popup: &str,
    terminal: &str,
) -> Result<(), String> {
    let _ = app.global_shortcut().unregister_all();
    if !enabled {
        return Ok(());
    }

    let translate_shortcut = parse_shortcut(translate)?;
    let enhance_shortcut = parse_shortcut(enhance)?;
    let popup_shortcut = parse_shortcut(popup)?;
    let terminal_shortcut = parse_shortcut(terminal)?;

    let h_t = app.clone();
    app.global_shortcut()
        .on_shortcut(translate_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_t.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_translate(h).await {
                    eprintln!("Global translate error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register translate shortcut: {}", e))?;

    let h_e = app.clone();
    app.global_shortcut()
        .on_shortcut(enhance_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_e.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_enhance(h).await {
                    eprintln!("Global enhance error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register enhance shortcut: {}", e))?;

    let h_p = app.clone();
    app.global_shortcut()
        .on_shortcut(popup_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_p.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_popup(h).await {
                    eprintln!("Global popup error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register popup shortcut: {}", e))?;

    let h_term = app.clone();
    app.global_shortcut()
        .on_shortcut(terminal_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_term.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_terminal(h).await {
                    eprintln!("Global terminal error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register terminal shortcut: {}", e))?;

    Ok(())
}

fn should_restore_main_window(app: &tauri::AppHandle) -> bool {
    if let Some(main_window) = app.get_webview_window("main") {
        let was_minimized = main_window.is_minimized().unwrap_or(false);
        let was_visible = main_window.is_visible().unwrap_or(true);
        if was_visible && !was_minimized {
            let _ = main_window.minimize();
            return true;
        }
    }
    false
}

fn show_loading_window(app: &tauri::AppHandle, payload: &str) {
    if let Some(window) = app.get_webview_window("loading") {
        let _ = window.emit("loading-update", payload);
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = window.center();
    }
}

fn hide_loading_window(app: &tauri::AppHandle, restore_main_window: bool) {
    if let Some(window) = app.get_webview_window("loading") {
        let _ = window.hide();
    }

    if restore_main_window {
        if let Some(main_window) = app.get_webview_window("main") {
            let _ = main_window.unminimize();
            let _ = main_window.set_focus();
        }
    }
}

fn current_config(state: &tauri::State<'_, Arc<AppState>>) -> crate::domain::llm_config::LLMConfig {
    state.last_config.lock().unwrap().clone()
}

fn current_languages(state: &tauri::State<'_, Arc<AppState>>) -> (String, String) {
    let source = state.preferred_source.lock().unwrap().clone();
    let target = state.preferred_target.lock().unwrap().clone();
    let source = if source.trim().is_empty() {
        "Auto Detect".to_string()
    } else {
        source
    };
    let target = if target.trim().is_empty() {
        "English".to_string()
    } else {
        target
    };
    (source, target)
}

async fn perform_robust_copy(enigo: &mut Enigo) {
    #[cfg(target_os = "windows")]
    {
        enigo.key_up(Key::Alt);
        enigo.key_up(Key::Control);
        tokio::time::sleep(Duration::from_millis(50)).await;

        enigo.key_down(Key::Control);
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_down(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Control);
    }
    #[cfg(target_os = "macos")]
    {
        enigo.key_up(Key::Option);
        enigo.key_up(Key::Meta);
        tokio::time::sleep(Duration::from_millis(50)).await;

        enigo.key_down(Key::Meta);
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_down(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Meta);
    }
    let _ = enigo;
}

async fn capture_selection<R: tauri::Runtime>(
    clipboard: &Clipboard<R>,
    enigo: &mut Enigo,
) -> Result<String, String> {
    let _ = clipboard.write_text("");
    perform_robust_copy(enigo).await;

    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(content) = clipboard.read_text() {
            if !content.is_empty() {
                return Ok(content);
            }
        }
    }

    Err("Clipboard is empty after auto-copy retries.".to_string())
}

async fn auto_paste(enigo: &mut Enigo) {
    #[cfg(target_os = "windows")]
    {
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Control);
    }
    #[cfg(target_os = "macos")]
    {
        enigo.key_down(Key::Meta);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Meta);
    }
    let _ = enigo;
}

async fn handle_global_translate(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    let restore_main_window = should_restore_main_window(&app);
    show_loading_window(&app, "translate");

    emit_shortcut_event(&app, "shortcut-start", "translate");
    log_shortcut(&state, "INFO", "Processing translate shortcut...");

    let mut enigo = Enigo::new();

    let text = match capture_selection(&clipboard, &mut enigo).await {
        Ok(text) => text,
        Err(message) => {
            log_shortcut(&state, "WARN", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    let config = current_config(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Shortcut config: provider={:?} base_url={} model={}",
            config.provider, config.base_url, config.model
        ),
    );

    let (source_lang, target_lang) = current_languages(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!("Shortcut languages: {} -> {}", source_lang, target_lang),
    );
    let result = match state
        .translate_use_case
        .execute(&config, text, source_lang, target_lang)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let message = e.to_string();
            log_shortcut(&state, "ERROR", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    if let Some(translated) = result.result {
        clipboard
            .write_text(translated)
            .map_err(|e| e.to_string())?;

        auto_paste(&mut enigo).await;
        log_shortcut(&state, "INFO", "Translation complete.");
    }

    emit_shortcut_event(&app, "shortcut-end", "success");
    hide_loading_window(&app, restore_main_window);
    Ok(())
}

async fn handle_global_enhance(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    let restore_main_window = should_restore_main_window(&app);
    show_loading_window(&app, "enhance");

    emit_shortcut_event(&app, "shortcut-start", "enhance");
    log_shortcut(&state, "INFO", "Processing enhance shortcut...");

    let mut enigo = Enigo::new();

    let text = match capture_selection(&clipboard, &mut enigo).await {
        Ok(text) => text,
        Err(message) => {
            log_shortcut(&state, "WARN", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    let config = current_config(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Shortcut config: provider={:?} base_url={} model={}",
            config.provider, config.base_url, config.model
        ),
    );

    let result = match state.enhance_use_case.execute(&config, text, None).await {
        Ok(result) => result,
        Err(e) => {
            let message = e.to_string();
            log_shortcut(&state, "ERROR", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    if let Some(enhanced) = result.result {
        clipboard.write_text(enhanced).map_err(|e| e.to_string())?;

        auto_paste(&mut enigo).await;
        log_shortcut(&state, "INFO", "Enhancement complete.");
    }

    emit_shortcut_event(&app, "shortcut-end", "success");
    hide_loading_window(&app, restore_main_window);
    Ok(())
}

async fn handle_global_popup(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    log_shortcut(&state, "INFO", "Processing popup shortcut...");
    let restore_main_window = should_restore_main_window(&app);
    emit_shortcut_event(&app, "shortcut-start", "popup");

    let mut enigo = Enigo::new();

    let text = capture_selection(&clipboard, &mut enigo).await.ok();

    if let Some(text) = text {
        log_shortcut(&state, "INFO", "Captured text for popup.");
        emit_shortcut_event(&app, "shortcut-capture", &text);
    } else {
        log_shortcut(&state, "WARN", "No text captured for popup.");
    }

    if restore_main_window {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    emit_shortcut_event(&app, "shortcut-end", "success");

    Ok(())
}

async fn handle_global_terminal(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    let restore_main_window = should_restore_main_window(&app);
    show_loading_window(&app, "terminal");

    emit_shortcut_event(&app, "shortcut-start", "terminal");
    log_shortcut(&state, "INFO", "Processing terminal shortcut...");

    let mut enigo = Enigo::new();

    let text = match capture_selection(&clipboard, &mut enigo).await {
        Ok(text) => text,
        Err(message) => {
            log_shortcut(&state, "WARN", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    let config = current_config(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Terminal shortcut config: provider={:?} base_url={} model={}",
            config.provider, config.base_url, config.model
        ),
    );

    let (source_lang, target_lang) = current_languages(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Terminal shortcut languages: {} -> {}",
            source_lang, target_lang
        ),
    );
    let result = match state
        .translate_use_case
        .execute(&config, text, source_lang, target_lang)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let message = e.to_string();
            log_shortcut(&state, "ERROR", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    if let Some(translated) = result.result {
        clipboard
            .write_text(translated)
            .map_err(|e| e.to_string())?;

        // NOTE: No auto-paste for terminal mode - user manually pastes when ready
        log_shortcut(
            &state,
            "INFO",
            "Terminal translation complete (clipboard ready, no auto-paste).",
        );
    }

    emit_shortcut_event(&app, "shortcut-end", "success");
    hide_loading_window(&app, restore_main_window);
    Ok(())
}
