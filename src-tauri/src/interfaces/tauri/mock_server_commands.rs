use std::sync::Arc;

use tauri::State;

use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use crate::interfaces::mock_server::{
    build_status as build_mock_status, save_config as save_mock_server_config, start_mock_server,
    stop_mock_server, MockServerConfig, MockServerStatus,
};

use super::state::AppState;


#[tauri::command]
pub async fn mock_server_get_config(state: State<'_, Arc<AppState>>) -> Result<MockServerConfig> {
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        "Mock server config requested",
    );
    let config = state.mock_server.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
pub async fn mock_server_update_config(
    state: State<'_, Arc<AppState>>,
    config: MockServerConfig,
) -> Result<MockServerConfig> {
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        "Mock server config updating...",
    );
    {
        let mut current = state.mock_server.config.lock().unwrap();
        *current = config.clone();
    } // Release the lock before saving to avoid deadlock
    save_mock_server_config(&state.mock_server)?;
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        "Mock server config saved successfully",
    );
    Ok(config)
}

#[tauri::command]
pub async fn mock_server_start(state: State<'_, Arc<AppState>>) -> Result<MockServerStatus> {
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        "Mock server start requested",
    );
    start_mock_server(state.mock_server.clone()).await?;
    Ok(build_mock_status(&state.mock_server))
}

#[tauri::command]
pub async fn mock_server_stop(state: State<'_, Arc<AppState>>) -> Result<MockServerStatus> {
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        "Mock server stop requested",
    );
    stop_mock_server(state.mock_server.clone()).await?;
    Ok(build_mock_status(&state.mock_server))
}

#[tauri::command]
pub async fn mock_server_status(state: State<'_, Arc<AppState>>) -> Result<MockServerStatus> {
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        "Mock server status requested",
    );
    Ok(build_mock_status(&state.mock_server))
}

