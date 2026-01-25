//! Document Management Commands
//!
//! This module provides Tauri commands for:
//! - Getting, listing, and deleting documents
//! - Importing files into collections
//! - Reindexing documents and collections

use crate::domain::error::Result;
use crate::domain::rag_entities::RagDocument;
use crate::interfaces::http::add_log;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

#[tauri::command]
pub async fn rag_get_document(
    state: State<'_, Arc<super::AppState>>,
    id: i64,
) -> Result<RagDocument> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Fetching document: {}", id),
    );

    state.rag_repository.get_document(id).await.map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Failed to fetch document: {}", e),
        );
        e
    })
}


#[tauri::command]
pub async fn rag_delete_document(state: State<'_, Arc<super::AppState>>, id: i64) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting document: {}", id),
    );

    let rows = state
        .rag_repository
        .delete_document(id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to delete document: {}", e),
            );
            e
        })?;

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleted document: {} (rows {})", id, rows),
    );

    Ok(rows)
}


#[tauri::command]
pub async fn rag_list_documents(
    state: State<'_, Arc<super::AppState>>,
    collection_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<RagDocument>> {
    add_log(&state.logs, "INFO", "RAG", "Listing documents");

    state
        .rag_repository
        .list_documents(collection_id, limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list documents: {}", e),
            );
            e
        })
}


#[tauri::command]
pub async fn rag_import_file(
    state: State<'_, Arc<super::AppState>>,
    file_path: String,
    collection_id: Option<i64>,
) -> Result<RagDocument> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Importing file: {}", file_path),
    );

    // CRITICAL SECURITY CHECK: Block file imports for DB collections
    if let Some(coll_id) = collection_id {
        let collection = state.rag_repository.get_collection(coll_id).await?;
        if collection.kind == crate::domain::rag_entities::CollectionKind::Db {
            let err_msg = format!(
                "File import blocked: Collection '{}' (id={}) is a Database Collection. \
                DB Collections are specialized for database queries only and cannot be used with files.",
                collection.name, coll_id
            );
            add_log(&state.logs, "WARN", "RAG", &err_msg);
            return Err(crate::domain::error::AppError::ValidationError(err_msg));
        }
    }

    let start = Instant::now();
    let result = state
        .rag_ingestion_use_case
        .ingest_file(&file_path, collection_id, state.logs.clone())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to import file: {}", e),
            );
            e
        });

    let doc_type = Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown");
    state.analytics_logger.log_extraction(
        doc_type,
        result.is_ok(),
        start.elapsed().as_millis() as u64,
    );

    result
}


#[tauri::command]
pub async fn rag_reindex_document(
    state: State<'_, Arc<super::AppState>>,
    document_id: i64,
) -> Result<ReindexProgress> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Re-indexing document: {}", document_id),
    );

    // Get all chunks for the document
    let chunks = state.rag_repository.get_chunks(document_id, 100000).await?;

    let total_chunks = chunks.len();
    let mut success_count = 0;
    let mut failed_count = 0;

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Re-indexing {} chunks for document {} with new model",
            total_chunks, document_id
        ),
    );

    for (index, chunk) in chunks.iter().enumerate() {
        match state
            .embedding_service
            .generate_embedding(&chunk.content)
            .await
        {
            Ok(embedding) => {
                let embedding_bytes = crate::application::use_cases::embedding_service::EmbeddingService::embedding_to_bytes(
                    &embedding,
                );
                if state
                    .rag_repository
                    .update_chunk_embedding(chunk.id, &embedding_bytes)
                    .await
                    .is_ok()
                {
                    success_count += 1;
                } else {
                    failed_count += 1;
                }
            }
            Err(e) => {
                add_log(
                    &state.logs,
                    "WARN",
                    "RAG",
                    &format!("Failed to embed chunk {}: {}", chunk.id, e),
                );
                failed_count += 1;
            }
        }

        // Log progress every 10 chunks
        if (index + 1) % 10 == 0 {
            add_log(
                &state.logs,
                "INFO",
                "RAG",
                &format!(
                    "Re-indexing progress: {}/{} chunks processed",
                    index + 1,
                    total_chunks
                ),
            );
        }
    }

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Document {} re-index complete: {} succeeded, {} failed",
            document_id, success_count, failed_count
        ),
    );

    Ok(ReindexProgress {
        document_id,
        total_chunks: total_chunks as i64,
        processed_chunks: (success_count + failed_count) as i64,
        success_count: success_count as i64,
        failed_count: failed_count as i64,
        current_dimension: state.embedding_service.get_current_dimension(),
    })
}

/// Re-index all documents in a collection

#[tauri::command]
pub async fn rag_reindex_collection(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> Result<CollectionReindexResult> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Re-indexing collection: {}", collection_id),
    );

    let documents = state
        .rag_repository
        .list_documents(Some(collection_id), 100000)
        .await?;

    let mut document_results = Vec::new();
    let mut total_chunks = 0;
    let mut total_success = 0;
    let mut total_failed = 0;

    for doc in &documents {
        match rag_reindex_document(state.clone(), doc.id).await {
            Ok(progress) => {
                total_chunks += progress.total_chunks;
                total_success += progress.success_count;
                total_failed += progress.failed_count;
                document_results.push(progress);
            }
            Err(e) => {
                add_log(
                    &state.logs,
                    "WARN",
                    "RAG",
                    &format!("Failed to re-index document {}: {}", doc.id, e),
                );
            }
        }
    }

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Collection {} re-index complete: {}/{} chunks successful",
            collection_id, total_success, total_chunks
        ),
    );

    Ok(CollectionReindexResult {
        collection_id,
        total_documents: documents.len() as i64,
        document_results,
        total_chunks,
        total_success,
        total_failed,
    })
}

#[derive(Debug, Serialize)]
pub struct ReindexProgress {
    pub document_id: i64,
    pub total_chunks: i64,
    pub processed_chunks: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub current_dimension: usize,
}

#[derive(Debug, Serialize)]
pub struct CollectionReindexResult {
    pub collection_id: i64,
    pub total_documents: i64,
    pub document_results: Vec<ReindexProgress>,
    pub total_chunks: i64,
    pub total_success: i64,
    pub total_failed: i64,
}

// Filter chunks by quality threshold

