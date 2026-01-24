use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use enigo::{Enigo, Key, KeyboardControllable};
use tauri::{Emitter, Manager};
use tauri_plugin_clipboard_manager::{Clipboard, ClipboardExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::interfaces::tauri::AppState;

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
            other => return Err(format!("Unknown modifier: {other}")),
        }
    }

    let code = parse_code(key_part)?;
    let modifiers = if modifiers.is_empty() { None } else { Some(modifiers) };
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
                    _ => return Err(format!("Unsupported key: {key}")),
                }
            }
        }
        _ if key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit()) => key.to_string(),
        _ => key.to_string(),
    };

    Code::from_str(&normalized).map_err(|_| format!("Unsupported key: {key}"))
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
        .map_err(|e| format!("Failed to register translate shortcut: {e}"))?;

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
        .map_err(|e| format!("Failed to register enhance shortcut: {e}"))?;

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
        .map_err(|e| format!("Failed to register popup shortcut: {e}"))?;

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
        .map_err(|e| format!("Failed to register terminal shortcut: {e}"))?;

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
    let (source_lang, target_lang) = current_languages(&state);

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
    let (source_lang, target_lang) = current_languages(&state);

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

        // NOTE: No auto-paste for terminal mode - user manually pastes when ready.
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
