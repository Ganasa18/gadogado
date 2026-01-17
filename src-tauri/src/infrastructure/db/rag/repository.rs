use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{
    CollectionQualityMetrics, DocumentWarning, DocumentWarningInput, RagCollection,
    RagCollectionInput, RagDocument, RagDocumentChunk, RagDocumentChunkInput, RagDocumentInput,
    RagExcelData, RagExcelDataInput, RetrievalGap, RetrievalGapInput,
};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub struct RagRepository {
    pool: SqlitePool,
}

impl RagRepository {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let db_url = db_path_to_url(db_path)?;
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse RAG DB URL: {}", e)))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to connect RAG DB: {}", e)))?;

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool for direct queries
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create_collection(&self, input: &RagCollectionInput) -> Result<RagCollection> {
        let result = sqlx::query_as::<_, RagCollectionEntity>(
            "INSERT INTO collections (name, description) VALUES (?, ?) RETURNING *",
        )
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create collection: {}", e)))?;

        Ok(result.into())
    }

    pub async fn get_collection(&self, id: i64) -> Result<RagCollection> {
        let collection = sqlx::query_as::<_, RagCollectionEntity>(
            "SELECT id, name, description, created_at FROM collections WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch collection: {}", e)))?;

        match collection {
            Some(collection) => Ok(collection.into()),
            None => Err(AppError::NotFound(format!("Collection not found: {}", id))),
        }
    }

    pub async fn list_collections(&self, limit: i64) -> Result<Vec<RagCollection>> {
        let collections = sqlx::query_as::<_, RagCollectionEntity>(
            "SELECT id, name, description, created_at FROM collections ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list collections: {}", e)))?;

        Ok(collections.into_iter().map(|c| c.into()).collect())
    }

    pub async fn delete_collection(&self, id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete collection: {}", e)))?;

        Ok(result.rows_affected())
    }

    pub async fn delete_document(&self, id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete document: {}", e)))?;

        Ok(result.rows_affected())
    }

    pub async fn create_document(&self, input: &RagDocumentInput) -> Result<RagDocument> {
        let result = sqlx::query_as::<_, RagDocumentEntity>(
            "INSERT INTO documents (collection_id, file_name, file_path, file_type, language, total_pages)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.collection_id)
        .bind(&input.file_name)
        .bind(&input.file_path)
        .bind(&input.file_type)
        .bind(input.language.as_deref().unwrap_or("auto"))
        .bind(input.total_pages.unwrap_or(1))
        .fetch_one(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create document: {}", e)))?;

        Ok(result.into())
    }

    pub async fn get_document(&self, id: i64) -> Result<RagDocument> {
        let document = sqlx::query_as::<_, RagDocumentEntity>(
            "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,
                    quality_score, ocr_confidence, chunk_count, warning_count, created_at
             FROM documents WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch document: {}", e)))?;

        match document {
            Some(document) => Ok(document.into()),
            None => Err(AppError::NotFound(format!("Document not found: {}", id))),
        }
    }

    pub async fn list_documents(&self, collection_id: Option<i64>, limit: i64) -> Result<Vec<RagDocument>> {
        if let Some(collection_id) = collection_id {
            let documents = sqlx::query_as::<_, RagDocumentEntity>(
                "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,
                        quality_score, ocr_confidence, chunk_count, warning_count, created_at
                 FROM documents WHERE collection_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(collection_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list documents: {}", e)))?;
            Ok(documents.into_iter().map(|d| d.into()).collect())
        } else {
            let documents = sqlx::query_as::<_, RagDocumentEntity>(
                "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,
                        quality_score, ocr_confidence, chunk_count, warning_count, created_at
                 FROM documents ORDER BY created_at DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list documents: {}", e)))?;
            Ok(documents.into_iter().map(|d| d.into()).collect())
        }
    }

    pub async fn create_chunk(&self, input: &RagDocumentChunkInput) -> Result<RagDocumentChunk> {
        let result = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "INSERT INTO document_chunks (doc_id, content, page_number, page_offset, chunk_index, token_count)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
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
            .map_err(|e| AppError::DatabaseError(format!("Failed to update chunk embedding: {}", e)))?;

        Ok(())
    }

    pub async fn get_chunks_with_embeddings(&self, doc_id: i64) -> Result<Vec<(i64, String, Option<i64>, Option<i64>, Option<Vec<f32>>)>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, page_offset, chunk_index, token_count,
                    chunk_quality, content_type, embedding_api
             FROM document_chunks WHERE doc_id = ? ORDER BY chunk_index ASC",
        )
        .bind(doc_id)
        .fetch_all(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch chunks with embeddings: {}", e)))?;

        let mut results = Vec::new();
        for chunk in chunks {
            let embedding = chunk.embedding_api
                .and_then(|bytes| crate::application::use_cases::embedding_service::EmbeddingService::bytes_to_embedding(&bytes).ok());
            results.push((chunk.id, chunk.content, chunk.page_number, chunk.page_offset, embedding));
        }

        Ok(results)
    }

    pub async fn get_chunks(&self, doc_id: i64, limit: i64) -> Result<Vec<RagDocumentChunk>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, page_offset, chunk_index, token_count,
                    chunk_quality, content_type, embedding_api
             FROM document_chunks WHERE doc_id = ? ORDER BY chunk_index ASC LIMIT ?",
        )
        .bind(doc_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch chunks: {}", e)))?;

        Ok(chunks.into_iter().map(|c| c.into()).collect())
    }

    pub async fn create_excel_data(&self, input: &RagExcelDataInput) -> Result<RagExcelData> {
        let result = sqlx::query_as::<_, RagExcelDataEntity>(
            "INSERT INTO excel_data (doc_id, row_index, data_json, val_a, val_b, val_c)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.doc_id)
        .bind(input.row_index)
        .bind(&input.data_json)
        .bind(&input.val_a)
        .bind(&input.val_b)
        .bind(input.val_c)
        .fetch_one(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create excel data: {}", e)))?;

        Ok(result.into())
    }

    pub async fn get_excel_data(&self, doc_id: i64, limit: i64) -> Result<Vec<RagExcelData>> {
        let data = sqlx::query_as::<_, RagExcelDataEntity>(
            "SELECT id, doc_id, row_index, data_json, val_a, val_b, val_c
             FROM excel_data WHERE doc_id = ? ORDER BY row_index ASC LIMIT ?",
        )
        .bind(doc_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch excel data: {}", e)))?;

        Ok(data.into_iter().map(|d| d.into()).collect())
    }

    /// Search chunks by collection with page metadata and document name
    pub async fn search_chunks_by_collection(
        &self,
        collection_id: i64,
        limit: i64,
    ) -> Result<Vec<ChunkWithMetadata>> {
        let chunks = sqlx::query_as::<_, ChunkWithDocEntity>(
            "SELECT dc.id, dc.doc_id, dc.content, dc.page_number, dc.page_offset, dc.chunk_index,
                    dc.token_count, dc.chunk_quality, dc.content_type, dc.embedding_api, d.file_name
             FROM document_chunks dc
             INNER JOIN documents d ON dc.doc_id = d.id
             WHERE d.collection_id = ?
             ORDER BY dc.chunk_index ASC
             LIMIT ?",
        )
        .bind(collection_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to search chunks by collection: {}", e)))?;

        let mut results = Vec::new();
        for chunk in chunks {
            let embedding = chunk.embedding_api
                .and_then(|bytes| crate::application::use_cases::embedding_service::EmbeddingService::bytes_to_embedding(&bytes).ok());
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

    pub async fn search_excel_by_collection(
        &self,
        collection_id: i64,
        limit: i64,
    ) -> Result<Vec<RagExcelData>> {
        let data = sqlx::query_as::<_, RagExcelDataEntity>(
            "SELECT ed.id, ed.doc_id, ed.row_index, ed.data_json, ed.val_a, ed.val_b, ed.val_c
             FROM excel_data ed
             INNER JOIN documents d ON ed.doc_id = d.id
             WHERE d.collection_id = ?
             ORDER BY ed.row_index ASC
             LIMIT ?",
        )
        .bind(collection_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to search excel by collection: {}", e)))?;

        Ok(data.into_iter().map(|d| d.into()).collect())
    }

    pub async fn search_excel_by_collection_with_filter(
        &self,
        collection_id: i64,
        column_a: Option<&str>,
        column_b: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RagExcelData>> {
        let mut query = String::from(
            "SELECT ed.id, ed.doc_id, ed.row_index, ed.data_json, ed.val_a, ed.val_b, ed.val_c
             FROM excel_data ed
             INNER JOIN documents d ON ed.doc_id = d.id
             WHERE d.collection_id = ?",
        );
        let mut params: Vec<String> = Vec::new();
        let mut param_count = 1;

        if let Some(val) = column_a {
            param_count += 1;
            query.push_str(&format!(" AND ed.val_a = ?{}", param_count));
            params.push(val.to_string());
        }

        if let Some(val) = column_b {
            param_count += 1;
            query.push_str(&format!(" AND ed.val_b = ?{}", param_count));
            params.push(val.to_string());
        }

        query.push_str(&format!(" ORDER BY ed.row_index ASC LIMIT ?{}", param_count + 1));

        let mut query_builder = sqlx::query_as::<_, RagExcelDataEntity>(&query);
        query_builder = query_builder.bind(collection_id);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        query_builder = query_builder.bind(limit);

        let data = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to search excel with filter: {}", e)))?;

        Ok(data.into_iter().map(|d| d.into()).collect())
    }

    /// Get a single chunk by ID
    pub async fn get_chunk(&self, chunk_id: i64) -> Result<RagDocumentChunk> {
        let chunk = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, page_offset, chunk_index, token_count,
                    chunk_quality, content_type, embedding_api
             FROM document_chunks WHERE id = ?",
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

        sqlx::query("UPDATE document_chunks SET content = ?, token_count = ?, embedding_api = NULL WHERE id = ?")
            .bind(new_content)
            .bind(token_count)
            .bind(chunk_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to update chunk content: {}", e)))?;

        Ok(())
    }

    // ============================================================
    // QUALITY ANALYTICS METHODS
    // ============================================================

    /// Update document quality metrics
    pub async fn update_document_quality(
        &self,
        doc_id: i64,
        quality_score: Option<f64>,
        ocr_confidence: Option<f64>,
        chunk_count: i64,
        warning_count: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE documents SET quality_score = ?, ocr_confidence = ?, chunk_count = ?, warning_count = ?
             WHERE id = ?",
        )
        .bind(quality_score)
        .bind(ocr_confidence)
        .bind(chunk_count)
        .bind(warning_count)
        .bind(doc_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update document quality: {}", e)))?;

        Ok(())
    }

    /// Update chunk quality
    pub async fn update_chunk_quality(
        &self,
        chunk_id: i64,
        quality: f64,
        content_type: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE document_chunks SET chunk_quality = ?, content_type = ? WHERE id = ?")
            .bind(quality)
            .bind(content_type)
            .bind(chunk_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to update chunk quality: {}", e)))?;

        Ok(())
    }

    /// Create a document warning
    pub async fn create_warning(&self, input: &DocumentWarningInput) -> Result<DocumentWarning> {
        let result = sqlx::query_as::<_, DocumentWarningEntity>(
            "INSERT INTO document_warnings (doc_id, warning_type, page_number, chunk_index, severity, message, suggestion)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.doc_id)
        .bind(&input.warning_type)
        .bind(input.page_number)
        .bind(input.chunk_index)
        .bind(&input.severity)
        .bind(&input.message)
        .bind(&input.suggestion)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create warning: {}", e)))?;

        Ok(result.into())
    }

    /// Get warnings for a document
    pub async fn get_document_warnings(&self, doc_id: i64) -> Result<Vec<DocumentWarning>> {
        let warnings = sqlx::query_as::<_, DocumentWarningEntity>(
            "SELECT id, doc_id, warning_type, page_number, chunk_index, severity, message, suggestion, created_at
             FROM document_warnings WHERE doc_id = ? ORDER BY created_at DESC",
        )
        .bind(doc_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get warnings: {}", e)))?;

        Ok(warnings.into_iter().map(|w| w.into()).collect())
    }

    /// Get collection quality metrics
    pub async fn get_collection_quality_metrics(
        &self,
        collection_id: i64,
    ) -> Result<Option<CollectionQualityMetrics>> {
        let result = sqlx::query_as::<_, CollectionQualityMetricsEntity>(
            "SELECT id, collection_id, computed_at, avg_quality_score, avg_ocr_confidence,
                    total_documents, documents_with_warnings, total_chunks, avg_chunk_quality,
                    best_reranker, reranker_score
             FROM collection_quality_metrics WHERE collection_id = ?
             ORDER BY computed_at DESC LIMIT 1",
        )
        .bind(collection_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get collection metrics: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    /// Compute and store collection quality metrics
    pub async fn compute_collection_quality_metrics(
        &self,
        collection_id: i64,
    ) -> Result<CollectionQualityMetrics> {
        // Aggregate metrics from documents
        let stats = sqlx::query_as::<_, CollectionStatsRow>(
            "SELECT
                COUNT(*) as total_documents,
                AVG(quality_score) as avg_quality_score,
                AVG(ocr_confidence) as avg_ocr_confidence,
                SUM(chunk_count) as total_chunks,
                SUM(CASE WHEN warning_count > 0 THEN 1 ELSE 0 END) as documents_with_warnings
             FROM documents WHERE collection_id = ?",
        )
        .bind(collection_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to compute collection stats: {}", e)))?;

        // Compute average chunk quality
        let avg_chunk_quality = sqlx::query_scalar::<_, Option<f64>>(
            "SELECT AVG(dc.chunk_quality) FROM document_chunks dc
             INNER JOIN documents d ON dc.doc_id = d.id
             WHERE d.collection_id = ?",
        )
        .bind(collection_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to compute chunk quality: {}", e)))?;

        // Insert the metrics
        let result = sqlx::query_as::<_, CollectionQualityMetricsEntity>(
            "INSERT INTO collection_quality_metrics
             (collection_id, avg_quality_score, avg_ocr_confidence, total_documents,
              documents_with_warnings, total_chunks, avg_chunk_quality)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(collection_id)
        .bind(stats.avg_quality_score)
        .bind(stats.avg_ocr_confidence)
        .bind(stats.total_documents)
        .bind(stats.documents_with_warnings)
        .bind(stats.total_chunks.unwrap_or(0))
        .bind(avg_chunk_quality)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store collection metrics: {}", e)))?;

        Ok(result.into())
    }

    /// Record a retrieval gap
    pub async fn record_retrieval_gap(&self, input: &RetrievalGapInput) -> Result<RetrievalGap> {
        let result = sqlx::query_as::<_, RetrievalGapEntity>(
            "INSERT INTO retrieval_gaps (collection_id, query_hash, query_length, result_count,
                                         max_confidence, avg_confidence, gap_type)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.collection_id)
        .bind(&input.query_hash)
        .bind(input.query_length)
        .bind(input.result_count)
        .bind(input.max_confidence)
        .bind(input.avg_confidence)
        .bind(&input.gap_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to record retrieval gap: {}", e)))?;

        Ok(result.into())
    }

    /// Get retrieval gaps for a collection
    pub async fn get_retrieval_gaps(
        &self,
        collection_id: i64,
        limit: i64,
    ) -> Result<Vec<RetrievalGap>> {
        let gaps = sqlx::query_as::<_, RetrievalGapEntity>(
            "SELECT id, collection_id, query_hash, query_length, result_count,
                    max_confidence, avg_confidence, gap_type, created_at
             FROM retrieval_gaps WHERE collection_id = ?
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(collection_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get retrieval gaps: {}", e)))?;

        Ok(gaps.into_iter().map(|g| g.into()).collect())
    }

    /// Get documents with quality below threshold
    pub async fn get_low_quality_documents(
        &self,
        collection_id: i64,
        threshold: f64,
        limit: i64,
    ) -> Result<Vec<RagDocument>> {
        let documents = sqlx::query_as::<_, RagDocumentEntity>(
            "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,
                    quality_score, ocr_confidence, chunk_count, warning_count, created_at
             FROM documents
             WHERE collection_id = ? AND (quality_score IS NULL OR quality_score < ?)
             ORDER BY quality_score ASC NULLS FIRST
             LIMIT ?",
        )
        .bind(collection_id)
        .bind(threshold)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get low quality documents: {}", e)))?;

        Ok(documents.into_iter().map(|d| d.into()).collect())
    }
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| AppError::DatabaseError("RAG database path is not valid UTF-8".to_string()))?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

#[derive(sqlx::FromRow)]
struct RagCollectionEntity {
    id: i64,
    name: String,
    description: Option<String>,
    created_at: String,
}

impl From<RagCollectionEntity> for RagCollection {
    fn from(entity: RagCollectionEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            description: entity.description,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct RagDocumentEntity {
    id: i64,
    collection_id: Option<i64>,
    file_name: String,
    file_path: Option<String>,
    file_type: String,
    language: String,
    total_pages: i64,
    quality_score: Option<f64>,
    ocr_confidence: Option<f64>,
    chunk_count: i64,
    warning_count: i64,
    created_at: String,
}

impl From<RagDocumentEntity> for RagDocument {
    fn from(entity: RagDocumentEntity) -> Self {
        Self {
            id: entity.id,
            collection_id: entity.collection_id,
            file_name: entity.file_name,
            file_path: entity.file_path,
            file_type: entity.file_type,
            language: entity.language,
            total_pages: entity.total_pages,
            quality_score: entity.quality_score,
            ocr_confidence: entity.ocr_confidence,
            chunk_count: entity.chunk_count,
            warning_count: entity.warning_count,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct RagDocumentChunkEntity {
    id: i64,
    doc_id: i64,
    content: String,
    page_number: Option<i64>,
    page_offset: Option<i64>,
    chunk_index: i64,
    token_count: Option<i64>,
    chunk_quality: Option<f64>,
    content_type: Option<String>,
    embedding_api: Option<Vec<u8>>,
}

impl From<RagDocumentChunkEntity> for RagDocumentChunk {
    fn from(entity: RagDocumentChunkEntity) -> Self {
        Self {
            id: entity.id,
            doc_id: entity.doc_id,
            content: entity.content,
            page_number: entity.page_number,
            page_offset: entity.page_offset,
            chunk_index: entity.chunk_index,
            token_count: entity.token_count,
            chunk_quality: entity.chunk_quality,
            content_type: entity.content_type,
        }
    }
}

/// Chunk entity with joined document metadata
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

/// Public struct for chunk search results with metadata
pub struct ChunkWithMetadata {
    pub id: i64,
    pub content: String,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub doc_name: String,
    pub embedding: Option<Vec<f32>>,
}

#[derive(sqlx::FromRow)]
struct RagExcelDataEntity {
    id: i64,
    doc_id: i64,
    row_index: i64,
    data_json: Option<String>,
    val_a: Option<String>,
    val_b: Option<String>,
    val_c: Option<f64>,
}

impl From<RagExcelDataEntity> for RagExcelData {
    fn from(entity: RagExcelDataEntity) -> Self {
        Self {
            id: entity.id,
            doc_id: entity.doc_id,
            row_index: entity.row_index,
            data_json: entity.data_json,
            val_a: entity.val_a,
            val_b: entity.val_b,
            val_c: entity.val_c,
        }
    }
}

// ============================================================
// QUALITY ANALYTICS ENTITY STRUCTS
// ============================================================

#[derive(sqlx::FromRow)]
struct DocumentWarningEntity {
    id: i64,
    doc_id: i64,
    warning_type: String,
    page_number: Option<i64>,
    chunk_index: Option<i64>,
    severity: String,
    message: String,
    suggestion: Option<String>,
    created_at: String,
}

impl From<DocumentWarningEntity> for DocumentWarning {
    fn from(entity: DocumentWarningEntity) -> Self {
        Self {
            id: entity.id,
            doc_id: entity.doc_id,
            warning_type: entity.warning_type,
            page_number: entity.page_number,
            chunk_index: entity.chunk_index,
            severity: entity.severity,
            message: entity.message,
            suggestion: entity.suggestion,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct CollectionQualityMetricsEntity {
    id: i64,
    collection_id: i64,
    computed_at: String,
    avg_quality_score: Option<f64>,
    avg_ocr_confidence: Option<f64>,
    total_documents: i64,
    documents_with_warnings: i64,
    total_chunks: i64,
    avg_chunk_quality: Option<f64>,
    best_reranker: Option<String>,
    reranker_score: Option<f64>,
}

impl From<CollectionQualityMetricsEntity> for CollectionQualityMetrics {
    fn from(entity: CollectionQualityMetricsEntity) -> Self {
        Self {
            id: entity.id,
            collection_id: entity.collection_id,
            computed_at: chrono::DateTime::parse_from_rfc3339(&entity.computed_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            avg_quality_score: entity.avg_quality_score,
            avg_ocr_confidence: entity.avg_ocr_confidence,
            total_documents: entity.total_documents,
            documents_with_warnings: entity.documents_with_warnings,
            total_chunks: entity.total_chunks,
            avg_chunk_quality: entity.avg_chunk_quality,
            best_reranker: entity.best_reranker,
            reranker_score: entity.reranker_score,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CollectionStatsRow {
    total_documents: i64,
    avg_quality_score: Option<f64>,
    avg_ocr_confidence: Option<f64>,
    total_chunks: Option<i64>,
    documents_with_warnings: i64,
}

#[derive(sqlx::FromRow)]
struct RetrievalGapEntity {
    id: i64,
    collection_id: i64,
    query_hash: String,
    query_length: Option<i64>,
    result_count: Option<i64>,
    max_confidence: Option<f64>,
    avg_confidence: Option<f64>,
    gap_type: Option<String>,
    created_at: String,
}

impl From<RetrievalGapEntity> for RetrievalGap {
    fn from(entity: RetrievalGapEntity) -> Self {
        Self {
            id: entity.id,
            collection_id: entity.collection_id,
            query_hash: entity.query_hash,
            query_length: entity.query_length,
            result_count: entity.result_count,
            max_confidence: entity.max_confidence,
            avg_confidence: entity.avg_confidence,
            gap_type: entity.gap_type,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}
