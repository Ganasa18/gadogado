use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{RagDocumentChunk, RagDocumentChunkInput};

use super::entities::RagDocumentChunkEntity;
use super::RagRepository;

/// Public struct for chunk search results with metadata
pub struct ChunkWithMetadata {
    pub id: i64,
    pub content: String,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub doc_name: String,
    pub embedding: Option<Vec<f32>>,
}

/// Chunk search result with a retrieval score.
///
/// Note: For FTS5, SQLite's `bm25()` returns lower-is-better values; callers should normalize.
pub struct ChunkWithMetadataScore {
    pub id: i64,
    pub content: String,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub doc_name: String,
    pub embedding: Option<Vec<f32>>,
    pub score: f32,
}

fn bytes_to_embedding(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }

    let mut embedding = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let arr = [chunk[0], chunk[1], chunk[2], chunk[3]];
        embedding.push(f32::from_le_bytes(arr));
    }

    Some(embedding)
}

impl RagRepository {
    pub async fn create_chunk(&self, input: &RagDocumentChunkInput) -> Result<RagDocumentChunk> {
        let result = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "INSERT INTO document_chunks (doc_id, content, page_number, page_offset, chunk_index, token_count)\n             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.doc_id)
        .bind(&input.content)
        .bind(input.page_number)
        .bind(input.page_offset)
        .bind(input.chunk_index)
        .bind(input.token_count)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create chunk: {}", e)))?;

        Ok(result.into())
    }

    pub async fn update_chunk_embedding(&self, chunk_id: i64, embedding: &[u8]) -> Result<()> {
        sqlx::query("UPDATE document_chunks SET embedding_api = ? WHERE id = ?")
            .bind(embedding)
            .bind(chunk_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to update chunk embedding: {}", e))
            })?;

        Ok(())
    }

    pub async fn get_chunks_with_embeddings(
        &self,
        doc_id: i64,
    ) -> Result<Vec<(i64, String, Option<i64>, Option<i64>, Option<Vec<f32>>)>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, page_offset, chunk_index, token_count,\n                    chunk_quality, content_type, embedding_api\n             FROM document_chunks WHERE doc_id = ? ORDER BY chunk_index ASC",
        )
        .bind(doc_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to fetch chunks with embeddings: {}", e))
        })?;

        let mut results = Vec::new();
        for chunk in chunks {
            let embedding = chunk
                .embedding_api
                .as_deref()
                .and_then(bytes_to_embedding);
            results.push((
                chunk.id,
                chunk.content,
                chunk.page_number,
                chunk.page_offset,
                embedding,
            ));
        }

        Ok(results)
    }

    pub async fn get_chunks(&self, doc_id: i64, limit: i64) -> Result<Vec<RagDocumentChunk>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, page_offset, chunk_index, token_count,\n                    chunk_quality, content_type, embedding_api\n             FROM document_chunks WHERE doc_id = ? ORDER BY chunk_index ASC LIMIT ?",
        )
        .bind(doc_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch chunks: {}", e)))?;

        Ok(chunks.into_iter().map(|c| c.into()).collect())
    }

    /// Search chunks by collection with page metadata and document name
    pub async fn search_chunks_by_collection(
        &self,
        collection_id: i64,
        limit: i64,
    ) -> Result<Vec<ChunkWithMetadata>> {
        #[derive(sqlx::FromRow)]
        struct ChunkWithDocEntity {
            id: i64,
            #[allow(dead_code)]
            doc_id: i64,
            content: String,
            page_number: Option<i64>,
            page_offset: Option<i64>,
            #[allow(dead_code)]
            chunk_index: i64,
            #[allow(dead_code)]
            token_count: Option<i64>,
            #[allow(dead_code)]
            chunk_quality: Option<f64>,
            #[allow(dead_code)]
            content_type: Option<String>,
            embedding_api: Option<Vec<u8>>,
            file_name: String,
        }

        let chunks = sqlx::query_as::<_, ChunkWithDocEntity>(
            "SELECT dc.id, dc.doc_id, dc.content, dc.page_number, dc.page_offset, dc.chunk_index,\n                    dc.token_count, dc.chunk_quality, dc.content_type, dc.embedding_api, d.file_name\n             FROM document_chunks dc\n             INNER JOIN documents d ON dc.doc_id = d.id\n             WHERE d.collection_id = ?\n             ORDER BY dc.chunk_index ASC\n             LIMIT ?",
        )
        .bind(collection_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to search chunks by collection: {}", e))
        })?;

        let mut results = Vec::new();
        for chunk in chunks {
            let embedding = chunk
                .embedding_api
                .as_deref()
                .and_then(bytes_to_embedding);
            results.push(ChunkWithMetadata {
                id: chunk.id,
                content: chunk.content,
                page_number: chunk.page_number,
                page_offset: chunk.page_offset,
                doc_name: chunk.file_name,
                embedding,
            });
        }

        Ok(results)
    }

    /// FTS5 keyword search over `document_chunks` scoped to a collection.
    ///
    /// This uses `document_chunks_fts` (rowid == document_chunks.id) and orders by
    /// `bm25(document_chunks_fts)` (ascending = best).
    pub async fn search_chunks_fts_by_collection(
        &self,
        collection_id: i64,
        query: &str,
        limit: i64,
    ) -> Result<Vec<ChunkWithMetadataScore>> {
        #[derive(sqlx::FromRow)]
        struct ChunkFtsEntity {
            id: i64,
            content: String,
            page_number: Option<i64>,
            page_offset: Option<i64>,
            embedding_api: Option<Vec<u8>>,
            file_name: String,
            bm25_score: f64,
        }

        let rows = sqlx::query_as::<_, ChunkFtsEntity>(
            "SELECT dc.id, dc.content, dc.page_number, dc.page_offset, dc.embedding_api, d.file_name,\n                    bm25(document_chunks_fts) AS bm25_score\n             FROM document_chunks_fts\n             INNER JOIN document_chunks dc ON dc.id = document_chunks_fts.rowid\n             INNER JOIN documents d ON dc.doc_id = d.id\n             WHERE d.collection_id = ?\n               AND document_chunks_fts MATCH ?\n             ORDER BY bm25_score ASC\n             LIMIT ?",
        )
        .bind(collection_id)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!(
                "Failed to search chunks (FTS) by collection: {}",
                e
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let embedding = row
                .embedding_api
                .as_deref()
                .and_then(bytes_to_embedding);

            results.push(ChunkWithMetadataScore {
                id: row.id,
                content: row.content,
                page_number: row.page_number,
                page_offset: row.page_offset,
                doc_name: row.file_name,
                embedding,
                score: row.bm25_score as f32,
            });
        }

        Ok(results)
    }

    /// Get a single chunk by ID
    pub async fn get_chunk(&self, chunk_id: i64) -> Result<RagDocumentChunk> {
        let chunk = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, page_offset, chunk_index, token_count,\n                    chunk_quality, content_type, embedding_api\n             FROM document_chunks WHERE id = ?",
        )
        .bind(chunk_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch chunk: {}", e)))?;

        Ok(chunk.into())
    }

    /// Delete a specific chunk
    pub async fn delete_chunk(&self, chunk_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM document_chunks WHERE id = ?")
            .bind(chunk_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete chunk: {}", e)))?;

        Ok(result.rows_affected())
    }

    /// Update chunk content
    pub async fn update_chunk_content(&self, chunk_id: i64, new_content: &str) -> Result<()> {
        let token_count = (new_content.len() / 4) as i64;

        sqlx::query(
            "UPDATE document_chunks SET content = ?, token_count = ?, embedding_api = NULL WHERE id = ?",
        )
        .bind(new_content)
        .bind(token_count)
        .bind(chunk_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update chunk content: {}", e)))?;

        Ok(())
    }
}
