use crate::application::use_cases::allowlist_validator::AllowlistValidator;
use crate::application::use_cases::audit_service::{AuditLogEntry, AuditService};
use crate::application::use_cases::chunking::{ChunkConfig, ChunkEngine, ChunkStrategy};
use crate::application::use_cases::data_protection::{ExternalLlmPolicy, LlmRoute};
use crate::application::use_cases::prompt_engine::{PromptEngine, VerificationResult};
use crate::application::use_cases::rag_analytics::{AnalyticsEvent, AnalyticsSummary};
use crate::application::use_cases::rag_config::{
    CacheConfig, ChatConfig, ChunkingConfig, ConfigValidation, EmbeddingConfig, FeedbackRating,
    FeedbackStats, OcrConfig, RagConfig, RetrievalConfig, UserFeedback,
};
use crate::application::use_cases::rag_ingestion::OcrResult;
use crate::application::use_cases::rag_validation::{
    RagValidationSuite, ValidationCase, ValidationOptions, ValidationReport,
};
use crate::application::use_cases::rate_limiter::{RateLimitResult, RateLimitStatus, RateLimiter};
use crate::application::use_cases::sql_compiler::{DbType, SqlCompiler};
use crate::application::use_cases::sql_rag_router::SqlRagRouter;
use crate::domain::error::Result;
use crate::domain::rag_entities::{
    DbAllowlistProfile, DbConnection, DbConnectionInput, RagCollection, RagCollectionInput,
    RagDocument, RagDocumentChunk, RagExcelData,
};
use crate::interfaces::http::add_log;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;


use super::types::*;

#[tauri::command]
pub async fn db_add_connection(
    state: State<'_, Arc<super::AppState>>,
    input: DbConnectionInput,
) -> Result<DbConnection> {
    add_log(
        &state.logs,
        "INFO",
        "DB",
        &format!("Adding DB connection: {}", input.name),
    );

    // For now, store the password directly as a reference
    // TODO: Implement secure storage using keychain
    let password_ref = format!(
        "env:DB_PASSWORD_{}",
        input.name.to_uppercase().replace(' ', "_")
    );

    state
        .rag_repository
        .create_db_connection(&input, &password_ref)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "DB",
                &format!("Failed to create DB connection: {}", e),
            );
            e
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
        "DB",
        &format!("Testing DB connection input: {}", input.name),
    );

    // Use the actual connection manager to test the connection
    let result = state
        .db_connection_manager
        .test_connection_input(&input)
        .await;

    add_log(
        &state.logs,
        if result.success { "INFO" } else { "WARN" },
        "DB",
        &format!(
            "Connection test for '{}': {} - {}",
            input.name,
            if result.success { "SUCCESS" } else { "FAILED" },
            result.message
        ),
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
        "DB",
        &format!("Testing DB connection: {}", conn_id),
    );

    // Get the connection config
    let conn_config = state
        .rag_repository
        .get_db_connection(conn_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "DB",
                &format!("Failed to get DB connection config: {}", e),
            );
            e
        })?;

    // Use the actual connection manager to test the connection
    let result = state
        .db_connection_manager
        .test_connection(&conn_config)
        .await;

    add_log(
        &state.logs,
        if result.success { "INFO" } else { "WARN" },
        "DB",
        &format!(
            "Connection test for '{}' (id={}): {} - {}",
            conn_config.name,
            conn_id,
            if result.success { "SUCCESS" } else { "FAILED" },
            result.message
        ),
    );

    Ok(result)
}

/// List all database connections

#[tauri::command]
pub async fn db_list_connections(
    state: State<'_, Arc<super::AppState>>,
) -> Result<Vec<DbConnection>> {
    add_log(&state.logs, "INFO", "DB", "Listing DB connections");

    state
        .rag_repository
        .list_db_connections()
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "DB",
                &format!("Failed to list DB connections: {}", e),
            );
            e
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
        "DB",
        &format!("Deleting DB connection: {}", conn_id),
    );

    state
        .rag_repository
        .delete_db_connection(conn_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "DB",
                &format!("Failed to delete DB connection: {}", e),
            );
            e
        })
}

/// List all allowlist profiles

#[tauri::command]
pub async fn db_list_allowlist_profiles(
    state: State<'_, Arc<super::AppState>>,
) -> Result<Vec<DbAllowlistProfile>> {
    add_log(&state.logs, "INFO", "DB", "Listing allowlist profiles");

    state
        .rag_repository
        .list_allowlist_profiles()
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "DB",
                &format!("Failed to list allowlist profiles: {}", e),
            );
            e
        })
}

/// Get allowlisted tables from a profile

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

    let profile = state
        .rag_repository
        .get_allowlist_profile(profile_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "DB",
                &format!("Failed to get allowlist profile: {}", e),
            );
            e
        })?;

    // Parse the rules_json to extract allowed tables
    let rules: serde_json::Value = serde_json::from_str(&profile.rules_json).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "DB",
            &format!("Failed to parse allowlist rules: {}", e),
        );
        crate::domain::error::AppError::ValidationError(format!("Invalid allowlist rules: {}", e))
    })?;

    let tables = rules["allowed_tables"]
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

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

