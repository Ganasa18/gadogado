//! Database Connection Management Commands
//!
//! This module provides Tauri commands for:
//! - Adding, testing, listing, and deleting database connections
//! - Managing allowlist profiles and selected tables
//! - Creating DB collections

use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{ColumnInfo, DbAllowlistProfile, DbConnection, DbConnectionInput, RagCollection, TableInfo};
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Constants
// ============================================================================

/// Log context for database operations
const LOG_CONTEXT_DB: &str = "DB";

/// Log context for RAG operations
const LOG_CONTEXT_RAG: &str = "RAG";

/// Prefix for password storage (plain text in DB for development)
/// TODO: Implement encryption or secure keychain storage for production
const PASSWORD_REF_PREFIX: &str = "plain:";

// ============================================================================
// Helper Functions
// ============================================================================

/// Generates a password reference string for storage in the database.
///
/// For development: stores password with "plain:" prefix directly in database.
/// TODO: Implement encryption or secure keychain storage for production.
fn generate_password_ref(password: &str) -> String {
    format!("{}{}", PASSWORD_REF_PREFIX, password)
}

/// Logs an error and returns the provided error
fn log_and_return_error(
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    context: &str,
    message: &str,
    error: crate::domain::error::AppError,
) -> crate::domain::error::AppError {
    add_log(logs, "ERROR", context, message);
    error
}

/// Logs operation success/failure based on result
fn log_operation_result(
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    context: &str,
    operation: &str,
    target: &str,
    success: bool,
    message: &str,
) {
    let level = if success { "INFO" } else { "WARN" };
    let status = if success { "SUCCESS" } else { "FAILED" };
    add_log(
        logs,
        level,
        context,
        &format!("{} for '{}': {} - {}", operation, target, status, message),
    );
}

#[tauri::command]
pub async fn db_add_connection(
    state: State<'_, Arc<super::AppState>>,
    input: DbConnectionInput,
) -> Result<DbConnection> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Adding DB connection: {}", input.name),
    );

    // Store password with "plain:" prefix directly in database
    // TODO: Implement encryption or secure keychain storage for production
    let password_ref = generate_password_ref(&input.password);

    state
        .rag_repository
        .create_db_connection(&input, &password_ref)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to create DB connection: {}", e),
                e,
            )
        })
}

/// Test a database connection input without saving

#[tauri::command]
pub async fn db_test_connection_input(
    state: State<'_, Arc<super::AppState>>,
    input: DbConnectionInput,
) -> Result<crate::domain::rag_entities::TestConnectionResult> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Testing DB connection input: {}", input.name),
    );

    let result = state
        .db_connection_manager
        .test_connection_input(&input)
        .await;

    log_operation_result(
        &state.logs,
        LOG_CONTEXT_DB,
        "Connection test",
        &input.name,
        result.success,
        &result.message,
    );

    Ok(result)
}

/// Test a database connection

#[tauri::command]
pub async fn db_test_connection(
    state: State<'_, Arc<super::AppState>>,
    conn_id: i64,
) -> Result<crate::domain::rag_entities::TestConnectionResult> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Testing DB connection: {}", conn_id),
    );

    let conn_config = state
        .rag_repository
        .get_db_connection(conn_id)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to get DB connection config: {}", e),
                e,
            )
        })?;

    let result = state
        .db_connection_manager
        .test_connection(&conn_config)
        .await;

    log_operation_result(
        &state.logs,
        LOG_CONTEXT_DB,
        "Connection test",
        &format!("{} (id={})", conn_config.name, conn_id),
        result.success,
        &result.message,
    );

    Ok(result)
}

/// List all database connections

#[tauri::command]
pub async fn db_list_connections(
    state: State<'_, Arc<super::AppState>>,
) -> Result<Vec<DbConnection>> {
    add_log(&state.logs, "INFO", LOG_CONTEXT_DB, "Listing DB connections");

    state
        .rag_repository
        .list_db_connections()
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to list DB connections: {}", e),
                e,
            )
        })
}

/// Delete a database connection

#[tauri::command]
pub async fn db_delete_connection(
    state: State<'_, Arc<super::AppState>>,
    conn_id: i64,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Deleting DB connection: {}", conn_id),
    );

    // Get connection first to get password_ref for keychain cleanup
    let conn = state.rag_repository.get_db_connection(conn_id).await.ok();

    // Delete from database
    let rows = state
        .rag_repository
        .delete_db_connection(conn_id)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to delete DB connection: {}", e),
                e,
            )
        })?;

    // Delete password from keychain (best effort, don't fail if this fails)
    if let Some(conn) = conn {
        if let Some(ref password_ref) = conn.password_ref {
            if password_ref.starts_with("keychain:") {
                let key_name = &password_ref[9..];
                let _ = crate::application::use_cases::db_connection_manager::DbConnectionManager::delete_password_from_keychain(key_name);
                add_log(
                    &state.logs,
                    "INFO",
                    LOG_CONTEXT_DB,
                    &format!("Cleaned up keychain entry for connection: {}", conn_id),
                );
            }
        }
    }

    Ok(rows)
}

/// List tables from a database connection
///
/// Returns a list of tables with their schema and approximate row counts.
/// Only queries metadata tables (information_schema) for security.

#[tauri::command]
pub async fn db_list_tables(
    state: State<'_, Arc<super::AppState>>,
    conn_id: i64,
) -> Result<Vec<TableInfo>> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Listing tables for connection: {}", conn_id),
    );

    let conn_config = state
        .rag_repository
        .get_db_connection(conn_id)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to get DB connection config: {}", e),
                e,
            )
        })?;

    let tables = state
        .db_connection_manager
        .list_tables(&conn_config)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to list tables: {}", e),
                e,
            )
        })?;

    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Found {} tables for connection {}", tables.len(), conn_id),
    );

    Ok(tables)
}

/// List columns for a specific table
///
/// Returns a list of columns with their data types, nullable status,
/// primary key status, and ordinal position.

#[tauri::command]
pub async fn db_list_columns(
    state: State<'_, Arc<super::AppState>>,
    conn_id: i64,
    table_name: String,
) -> Result<Vec<ColumnInfo>> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Listing columns for table: {} (connection: {})", table_name, conn_id),
    );

    let conn_config = state
        .rag_repository
        .get_db_connection(conn_id)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to get DB connection config: {}", e),
                e,
            )
        })?;

    let columns = state
        .db_connection_manager
        .list_columns(&conn_config, &table_name)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to list columns: {}", e),
                e,
            )
        })?;

    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Found {} columns for table {}", columns.len(), table_name),
    );

    Ok(columns)
}

/// List all allowlist profiles

#[tauri::command]
pub async fn db_list_allowlist_profiles(
    state: State<'_, Arc<super::AppState>>,
) -> Result<Vec<DbAllowlistProfile>> {
    add_log(&state.logs, "INFO", LOG_CONTEXT_DB, "Listing allowlist profiles");

    state
        .rag_repository
        .list_allowlist_profiles()
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to list allowlist profiles: {}", e),
                e,
            )
        })
}

/// Get allowlisted tables from a profile
///
/// If the requested profile doesn't exist, attempts to use the default profile (ID=1).
/// Returns an empty list if no profiles are found, allowing the frontend to handle gracefully.

#[tauri::command]
pub async fn db_list_allowlisted_tables(
    state: State<'_, Arc<super::AppState>>,
    profile_id: i64,
) -> Result<Vec<String>> {
    add_log(
        &state.logs,
        "INFO",
        "DB",
        &format!("Listing allowlisted tables for profile: {}", profile_id),
    );

    // Try to get the requested profile
    let profile_result = state
        .rag_repository
        .get_allowlist_profile(profile_id)
        .await;

    let profile = match profile_result {
        Ok(p) => p,
        Err(e) => {
            // If profile not found, try the default profile (ID=1)
            if profile_id != 1 {
                add_log(
                    &state.logs,
                    "WARN",
                    "DB",
                    &format!("Profile {} not found, trying default profile (ID=1): {}", profile_id, e),
                );
                match state.rag_repository.get_allowlist_profile(1).await {
                    Ok(p) => p,
                    Err(default_err) => {
                        add_log(
                            &state.logs,
                            "ERROR",
                            "DB",
                            &format!("No allowlist profiles available. Please restart the application to initialize the default profile: {}", default_err),
                        );
                        return Err(crate::domain::error::AppError::NotFound(
                            "No allowlist profiles found. Please restart the application to initialize the default profile.".to_string()
                        ));
                    }
                }
            } else {
                add_log(
                    &state.logs,
                    "ERROR",
                    "DB",
                    &format!("Default allowlist profile (ID=1) not found. Please restart the application: {}", e),
                );
                return Err(crate::domain::error::AppError::NotFound(
                    "Default allowlist profile not found. Please restart the application to initialize it.".to_string()
                ));
            }
        }
    };

    // Parse the rules_json to extract allowed tables
    let rules: serde_json::Value = serde_json::from_str(&profile.rules_json).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "DB",
            &format!("Failed to parse allowlist rules for profile {}: {}", profile.id, e),
        );
        crate::domain::error::AppError::ValidationError(format!("Invalid allowlist rules: {}", e))
    })?;

    let tables: Vec<String> = rules["allowed_tables"]
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    add_log(
        &state.logs,
        "INFO",
        "DB",
        &format!("Found {} allowlisted tables for profile {}", tables.len(), profile.id),
    );

    Ok(tables)
}

/// Create a DB collection with specified configuration

#[tauri::command]
pub async fn rag_create_db_collection(
    state: State<'_, Arc<super::AppState>>,
    name: String,
    description: Option<String>,
    config_json: String,
) -> Result<RagCollection> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Creating DB collection: {}", name),
    );

    state
        .rag_repository
        .create_collection_with_config(&name, &description.unwrap_or_default(), "db", &config_json)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to create DB collection: {}", e),
            );
            e
        })
}

/// Update selected tables for a DB collection

#[tauri::command]
pub async fn db_update_selected_tables(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    selected_tables: Vec<String>,
) -> Result<()> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Updating selected tables for collection: {}", collection_id),
    );

    // Get current collection config
    let collection = state
        .rag_repository
        .get_collection(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get collection: {}", e),
            );
            e
        })?;

    // Parse current config and update selected_tables
    let mut config: serde_json::Value =
        serde_json::from_str(&collection.config_json).map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to parse collection config: {}", e),
            );
            crate::domain::error::AppError::ValidationError(format!(
                "Invalid collection config: {}",
                e
            ))
        })?;

    config["selected_tables"] = serde_json::json!(selected_tables);

    state
        .rag_repository
        .update_collection_config(collection_id, &config.to_string())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to update collection config: {}", e),
            );
            e
        })
}

/// Get selected tables for a DB collection

#[tauri::command]
pub async fn db_get_selected_tables(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<Vec<String>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting selected tables for collection: {}", collection_id),
    );

    let collection = state
        .rag_repository
        .get_collection(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get collection: {}", e),
            );
            e
        })?;

    let config: serde_json::Value = serde_json::from_str(&collection.config_json).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Failed to parse collection config: {}", e),
        );
        crate::domain::error::AppError::ValidationError(format!("Invalid collection config: {}", e))
    })?;

    let selected_tables = config["selected_tables"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    Ok(selected_tables)
}

/// Create a new allowlist profile
#[tauri::command]
pub async fn db_create_allowlist_profile(
    state: State<'_, Arc<super::AppState>>,
    name: String,
    description: Option<String>,
    rules_json: String,
) -> Result<DbAllowlistProfile> {
    add_log(
        &state.logs,
        "INFO",
        "DB",
        &format!("Creating allowlist profile: {}", name),
    );

    state
        .rag_repository
        .create_allowlist_profile(&name, description.as_deref(), &rules_json)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to create profile: {}", e),
                e,
            )
        })
}

/// Update an existing allowlist profile
#[tauri::command]
pub async fn db_update_allowlist_profile(
    state: State<'_, Arc<super::AppState>>,
    profile_id: i64,
    name: Option<String>,
    description: Option<Option<String>>,
    rules_json: Option<String>,
) -> Result<DbAllowlistProfile> {
    add_log(
        &state.logs,
        "INFO",
        "DB",
        &format!("Updating allowlist profile: {}", profile_id),
    );

    // Map description from Option<Option<String>> to Option<Option<&str>>
    let description_ref = description.as_ref().map(|d| d.as_deref());

    state
        .rag_repository
        .update_allowlist_profile(
            profile_id,
            name.as_deref(),
            description_ref,
            rules_json.as_deref(),
        )
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to update profile: {}", e),
                e,
            )
        })
}

/// Delete an allowlist profile (default profile id=1 is protected)
#[tauri::command]
pub async fn db_delete_allowlist_profile(
    state: State<'_, Arc<super::AppState>>,
    profile_id: i64,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "DB",
        &format!("Deleting allowlist profile: {}", profile_id),
    );

    state
        .rag_repository
        .delete_allowlist_profile(profile_id)
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to delete profile: {}", e),
                e,
            )
        })
}


/// Save connection configuration (tables and columns)
#[tauri::command]
pub async fn db_save_connection_config(
    state: State<'_, Arc<super::AppState>>,
    conn_id: i64,
    profile_id: i64,
    selected_tables: Vec<String>,
    selected_columns: std::collections::HashMap<String, Vec<String>>,
) -> Result<DbConnection> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT_DB,
        &format!("Saving config for connection: {} ({} tables, {} column configs)",
            conn_id,
            selected_tables.len(),
            selected_columns.len()
        ),
    );

    // Build config JSON
    let config = serde_json::json!({
        "profile_id": profile_id,
        "selected_tables": selected_tables,
        "selected_columns": selected_columns,
        "updated_at": chrono::Utc::now().to_rfc3339()
    });

    state
        .rag_repository
        .update_db_connection_config(conn_id, &config.to_string())
        .await
        .map_err(|e| {
            log_and_return_error(
                &state.logs,
                LOG_CONTEXT_DB,
                &format!("Failed to save connection config: {}", e),
                e,
            )
        })
}

/// Get connection configuration
#[tauri::command]
pub async fn db_get_connection_config(
    state: State<'_, Arc<super::AppState>>,
    conn_id: i64,
) -> Result<serde_json::Value> {
    let conn = state
        .rag_repository
        .get_db_connection(conn_id)
        .await?;

    match conn.config_json {
        Some(config_json) => {
            Ok(serde_json::from_str(&config_json).map_err(|e| {
                AppError::ValidationError(format!("Invalid config JSON: {}", e))
            })?)
        }
        None => {
            // Return empty config if none exists
            Ok(serde_json::json!({
                "profile_id": null,
                "selected_tables": Vec::<String>::new(),
                "selected_columns": std::collections::HashMap::<String, Vec<String>>::new()
            }))
        }
    }
}
