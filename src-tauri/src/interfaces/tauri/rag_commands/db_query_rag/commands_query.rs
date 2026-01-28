use crate::domain::error::Result;
use crate::domain::rag_entities::DbConnectionConfig;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

use super::constants::DEFAULT_LIMIT;
use super::flow_execute::execute_and_build_response;
use super::flow_resolve::{resolve_limit_from_request, resolve_sql_and_plan};
use super::helpers::{check_rate_limit, parse_collection_config, truncate_query_for_log, CollectionConfig};
use super::super::types::DbQueryRequest;
use crate::application::use_cases::sql_rag_router::SqlRagRouter;

pub async fn db_query_rag_impl(
    state: State<'_, Arc<super::super::AppState>>,
    request: DbQueryRequest,
) -> Result<super::super::types::DbQueryResponse> {
    let start = Instant::now();

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Processing query for collection {}: {}",
            request.collection_id,
            truncate_query_for_log(&request.query)
        ),
    );

    let rate_limit_result = state
        .rate_limiter
        .check_rate_limit(request.collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Rate limit check failed: {}", e),
            );
            e
        })?;

    check_rate_limit(&rate_limit_result, &state.logs)?;

    let collection = match state
        .rag_repository
        .get_collection(request.collection_id)
        .await
    {
        Ok(col) => col,
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get collection: {}", e),
            );
            let _ = state.rate_limiter.record_block(request.collection_id).await;
            return Err(e);
        }
    };

    if !matches!(
        collection.kind,
        crate::domain::rag_entities::CollectionKind::Db
    ) {
        add_log(
            &state.logs,
            "ERROR",
            "SQL-RAG",
            "SQL-RAG query called on non-DB collection",
        );
        return Err(crate::domain::error::AppError::ValidationError(
            "This collection is not a DB collection. Use standard RAG query instead.".to_string(),
        ));
    }

    let config_json = parse_collection_config(&collection.config_json, &state.logs)?;
    let collection_config = CollectionConfig::from_json(&config_json)?;

    let db_conn = state
        .rag_repository
        .get_db_connection(collection_config.db_conn_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get DB connection: {}", e),
            );
            e
        })?;

    let allowlist_profile = state
        .rag_repository
        .get_allowlist_profile(collection_config.allowlist_profile_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get allowlist profile: {}", e),
            );
            e
        })?;

    let conn_config: DbConnectionConfig = if let Some(ref json) = db_conn.config_json {
        serde_json::from_str(json).unwrap_or_else(|_| DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        })
    } else {
        DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        }
    };

    let router = SqlRagRouter::from_profile(
        &allowlist_profile,
        collection_config.selected_tables.clone(),
        conn_config.selected_columns.clone(),
    )
    .map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "SQL-RAG",
            &format!("Failed to create SQL router: {}", e),
        );
        e
    })?;

    let effective_limit = resolve_limit_from_request(request.limit, conn_config.default_limit);
    if effective_limit <= 0 {
        return Err(crate::domain::error::AppError::ValidationError(
            "Limit must be > 0".to_string(),
        ));
    }

    let effective_limit = if effective_limit == 0 {
        DEFAULT_LIMIT
    } else {
        effective_limit
    };

    let app_state = state.inner().clone();

    let resolved = resolve_sql_and_plan(
        &app_state,
        &request,
        &collection_config,
        &conn_config,
        &allowlist_profile,
        &router,
        &db_conn,
        effective_limit,
    )
    .await?;

    execute_and_build_response(&app_state, &request, &db_conn, resolved, start).await
}
