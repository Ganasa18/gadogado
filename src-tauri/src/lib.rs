mod application;
mod domain;
mod infrastructure;
mod interfaces;

mod app;

// Keep crate-level API stable: tauri commands call `crate::register_shortcuts`.
pub(crate) use crate::interfaces::shortcuts::register_shortcuts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
