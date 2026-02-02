//! Collection Management Commands
//!
//! This module provides Tauri commands for:
//! - Creating, reading, listing, and deleting RAG collections
//! - Collection quality metrics computation

use crate::domain::error::Result;
use crate::domain::rag_entities::{RagCollection, RagCollectionInput};
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

use super::types::CollectionQualityMetrics;

#[tauri::command]
pub async fn rag_create_collection(
    state: State<'_, Arc<super::AppState>>,
    input: RagCollectionInput,
) -> Result<RagCollection> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Creating collection: {}", input.name),
    );

    state
        .rag_repository
        .create_collection(&input)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to create collection: {}", e),
            );
            e
        })
}


#[tauri::command]
pub async fn rag_get_collection(
    state: State<'_, Arc<super::AppState>>,
    id: i64,
) -> Result<RagCollection> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Fetching collection: {}", id),
    );

    state.rag_repository.get_collection(id).await.map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Failed to fetch collection: {}", e),
        );
        e
    })
}


#[tauri::command]
pub async fn rag_list_collections(
    state: State<'_, Arc<super::AppState>>,
    limit: Option<i64>,
) -> Result<Vec<RagCollection>> {
    add_log(&state.logs, "INFO", "RAG", "Listing collections");

    state
        .rag_repository
        .list_collections(limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list collections: {}", e),
            );
            e
        })
}


#[tauri::command]
pub async fn rag_delete_collection(state: State<'_, Arc<super::AppState>>, id: i64) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting collection: {}", id),
    );

    state
        .rag_repository
        .delete_collection(id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to delete collection: {}", e),
            );
            e
        })
}


#[tauri::command]
pub async fn rag_get_collection_quality(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<Option<CollectionQualityMetrics>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting quality metrics for collection {}", collection_id),
    );

    state
        .rag_repository
        .get_collection_quality_metrics(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to get collection quality: {}", e),
            );
            e
        })
}

/// Compute and refresh collection quality metrics

#[tauri::command]
pub async fn rag_compute_collection_quality(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<CollectionQualityMetrics> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Computing quality metrics for collection {}", collection_id),
    );

    state
        .rag_repository
        .compute_collection_quality_metrics(collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to compute collection quality: {}", e),
            );
            e
        })
}

// Get document warnings

