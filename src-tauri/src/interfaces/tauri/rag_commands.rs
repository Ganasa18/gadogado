use crate::application::use_cases::prompt_engine::PromptEngine;
use crate::domain::error::Result;
use crate::domain::rag_entities::{
    RagCollection, RagCollectionInput, RagDocument, RagDocumentChunk, RagExcelData,
};
use crate::interfaces::http::add_log;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct RagQueryRequest {
    pub collection_id: i64,
    pub query: String,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct RagQueryResponse {
    pub prompt: String,
    pub results: Vec<crate::application::QueryResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagWebImportRequest {
    pub url: String,
    pub collection_id: Option<i64>,
    pub max_pages: Option<usize>,
    pub max_depth: Option<usize>,
}

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
pub async fn rag_delete_document(
    state: State<'_, Arc<super::AppState>>,
    id: i64,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting document: {}", id),
    );

    let rows = state.rag_repository.delete_document(id).await.map_err(|e| {
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

    state
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
        })
}

#[tauri::command]
pub async fn rag_list_chunks(
    state: State<'_, Arc<super::AppState>>,
    doc_id: i64,
    limit: Option<i64>,
) -> Result<Vec<RagDocumentChunk>> {
    add_log(&state.logs, "INFO", "RAG", "Listing chunks");

    state
        .rag_repository
        .get_chunks(doc_id, limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list chunks: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_list_excel_data(
    state: State<'_, Arc<super::AppState>>,
    doc_id: i64,
    limit: Option<i64>,
) -> Result<Vec<RagExcelData>> {
    add_log(&state.logs, "INFO", "RAG", "Listing Excel data");

    state
        .rag_repository
        .get_excel_data(doc_id, limit.unwrap_or(50))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Failed to list Excel data: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_hybrid_search(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<crate::application::QueryResult>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Hybrid search in collection {}: {}", collection_id, query),
    );

    state
        .retrieval_service
        .query(collection_id, &query, top_k.unwrap_or(5))
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Hybrid search failed: {}", e),
            );
            e
        })
}

#[tauri::command]
pub async fn rag_query(
    state: State<'_, Arc<super::AppState>>,
    request: RagQueryRequest,
) -> Result<RagQueryResponse> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Query in collection {}: {}",
            request.collection_id, request.query
        ),
    );

    let top_k = request.top_k.unwrap_or(5);
    let results = state
        .retrieval_service
        .query(request.collection_id, &request.query, top_k)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Retrieval failed: {}", e),
            );
            e
        })?;

    let prompt = PromptEngine::build_prompt(&request.query, &results).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "RAG",
            &format!("Prompt building failed: {}", e),
        );
        e
    })?;

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Built prompt with {} results", results.len()),
    );

    Ok(RagQueryResponse { prompt, results })
}

#[tauri::command]
pub async fn rag_import_web(
    state: State<'_, Arc<super::AppState>>,
    request: RagWebImportRequest,
) -> Result<RagDocument> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Importing web: {}", request.url),
    );

    let file_path = request.url.clone();
    state
        .rag_ingestion_use_case
        .ingest_file(&file_path, request.collection_id, state.logs.clone())
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "RAG",
                &format!("Web import failed: {}", e),
            );
            e
        })
}
