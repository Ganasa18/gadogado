//! Chunk Management Commands
//!
//! This module provides Tauri commands for:
//! - Listing and filtering chunks
//! - Chunk quality analysis
//! - Chunk content editing and re-embedding
//! - Excel data listing

use crate::domain::error::Result;
use crate::domain::rag_entities::{RagDocumentChunk, RagExcelData};
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

use super::types::*;

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
pub async fn rag_get_chunks_with_quality(
    state: State<'_, Arc<super::AppState>>,
    document_id: i64,
    limit: Option<i64>,
) -> Result<Vec<ChunkWithQuality>> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Getting chunks with quality for document: {}", document_id),
    );

    // Get chunks with embeddings to check which have embeddings
    let chunks_with_embeddings = state
        .rag_repository
        .get_chunks_with_embeddings(document_id)
        .await?;

    // Create a set of chunk IDs that have embeddings
    let chunks_with_embedding_ids: std::collections::HashSet<i64> = chunks_with_embeddings
        .iter()
        .filter(|(_, _, _, _, emb)| emb.is_some())
        .map(|(id, _, _, _, _)| *id)
        .collect();

    let chunks = state
        .rag_repository
        .get_chunks(document_id, limit.unwrap_or(1000))
        .await?;

    let chunks_with_quality: Vec<ChunkWithQuality> = chunks
        .into_iter()
        .map(|chunk| {
            let quality_score = estimate_chunk_quality(&chunk.content);
            let has_embedding = chunks_with_embedding_ids.contains(&chunk.id);
            let token_estimate = chunk.content.len() / 4; // ~4 chars per token

            ChunkWithQuality {
                chunk,
                quality_score,
                has_embedding,
                token_estimate,
            }
        })
        .collect();

    Ok(chunks_with_quality)
}

/// Delete a specific chunk

#[tauri::command]
pub async fn rag_delete_chunk(
    state: State<'_, Arc<super::AppState>>,
    chunk_id: i64,
) -> Result<u64> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Deleting chunk: {}", chunk_id),
    );

    state.rag_repository.delete_chunk(chunk_id).await
}

/// Update chunk content (for manual editing)

#[tauri::command]
pub async fn rag_update_chunk_content(
    state: State<'_, Arc<super::AppState>>,
    chunk_id: i64,
    new_content: String,
) -> Result<RagDocumentChunk> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Updating chunk content: {}", chunk_id),
    );

    // Update content and regenerate embedding
    state
        .rag_repository
        .update_chunk_content(chunk_id, &new_content)
        .await?;

    // Regenerate embedding for the updated content
    if let Ok(embedding) = state
        .embedding_service
        .generate_embedding(&new_content)
        .await
    {
        let embedding_bytes =
            crate::application::use_cases::embedding_service::EmbeddingService::embedding_to_bytes(
                &embedding,
            );
        let _ = state
            .rag_repository
            .update_chunk_embedding(chunk_id, &embedding_bytes)
            .await;
    }

    // Return updated chunk
    state.rag_repository.get_chunk(chunk_id).await
}

/// Re-embed a chunk (regenerate embedding)

#[tauri::command]
pub async fn rag_reembed_chunk(
    state: State<'_, Arc<super::AppState>>,
    chunk_id: i64,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!("Re-embedding chunk: {}", chunk_id),
    );

    let chunk = state.rag_repository.get_chunk(chunk_id).await?;

    let embedding = state
        .embedding_service
        .generate_embedding(&chunk.content)
        .await
        .map_err(|e| {
            crate::domain::error::AppError::Internal(format!("Embedding failed: {}", e))
        })?;

    let embedding_bytes =
        crate::application::use_cases::embedding_service::EmbeddingService::embedding_to_bytes(
            &embedding,
        );
    state
        .rag_repository
        .update_chunk_embedding(chunk_id, &embedding_bytes)
        .await?;

    Ok("Chunk re-embedded successfully".to_string())
}

/// Re-index all chunks in a document with the new embedding model

#[tauri::command]
pub async fn rag_filter_low_quality_chunks(
    state: State<'_, Arc<super::AppState>>,
    document_id: i64,
    quality_threshold: f32,
) -> Result<Vec<ChunkWithQuality>> {
    let all_chunks = rag_get_chunks_with_quality(state.clone(), document_id, Some(10000)).await?;

    let low_quality: Vec<ChunkWithQuality> = all_chunks
        .into_iter()
        .filter(|c| c.quality_score < quality_threshold)
        .collect();

    add_log(
        &state.logs,
        "INFO",
        "RAG",
        &format!(
            "Found {} low quality chunks (threshold: {})",
            low_quality.len(),
            quality_threshold
        ),
    );

    Ok(low_quality)
}

// Helper function to estimate chunk quality (replicated from rag_ingestion)
fn estimate_chunk_quality(content: &str) -> f32 {
    let mut score = 0.0f32;
    let len = content.len();

    // Length score (prefer 100-500 chars)
    if len >= 100 && len <= 500 {
        score += 0.3;
    } else if len >= 50 && len <= 800 {
        score += 0.2;
    } else if len < 50 {
        score += 0.05;
    } else {
        score += 0.1;
    }

    // Content quality indicators
    let has_alphanumeric = content.chars().any(|c| c.is_alphanumeric());
    let alpha_ratio =
        content.chars().filter(|c| c.is_alphabetic()).count() as f32 / len.max(1) as f32;
    let has_sentences = content.contains('.') || content.contains('!') || content.contains('?');
    let has_capital = content.chars().any(|c| c.is_uppercase());

    if has_alphanumeric {
        score += 0.2;
    }
    if alpha_ratio > 0.5 {
        score += 0.2;
    }
    if has_sentences {
        score += 0.15;
    }
    if has_capital {
        score += 0.15;
    }

    score.min(1.0)
}

pub(crate) fn average_score(results: &[crate::application::QueryResult]) -> Option<f32> {
    let mut total = 0.0f32;
    let mut count = 0usize;

    for result in results {
        if let Some(score) = result.score {
            total += score;
            count += 1;
        }
    }

    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}

