use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{
    CollectionQualityMetrics, DocumentWarning, DocumentWarningInput, RagDocument, RetrievalGap,
    RetrievalGapInput,
};

use super::entities::{
    CollectionQualityMetricsEntity, DocumentWarningEntity, RagDocumentEntity, RetrievalGapEntity,
};
use super::RagRepository;

impl RagRepository {
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
            "UPDATE documents SET quality_score = ?, ocr_confidence = ?, chunk_count = ?, warning_count = ?\n             WHERE id = ?",
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
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to update chunk quality: {}", e))
            })?;

        Ok(())
    }

    /// Create a document warning
    pub async fn create_warning(&self, input: &DocumentWarningInput) -> Result<DocumentWarning> {
        let result = sqlx::query_as::<_, DocumentWarningEntity>(
            "INSERT INTO document_warnings (doc_id, warning_type, page_number, chunk_index, severity, message, suggestion)\n             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
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
            "SELECT id, doc_id, warning_type, page_number, chunk_index, severity, message, suggestion, created_at\n             FROM document_warnings WHERE doc_id = ? ORDER BY created_at DESC",
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
            "SELECT id, collection_id, computed_at, avg_quality_score, avg_ocr_confidence,\n                    total_documents, documents_with_warnings, total_chunks, avg_chunk_quality,\n                    best_reranker, reranker_score\n             FROM collection_quality_metrics WHERE collection_id = ?\n             ORDER BY computed_at DESC LIMIT 1",
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
        #[derive(sqlx::FromRow)]
        struct CollectionStatsRow {
            total_documents: i64,
            avg_quality_score: Option<f64>,
            avg_ocr_confidence: Option<f64>,
            total_chunks: Option<i64>,
            documents_with_warnings: i64,
        }

        // Aggregate metrics from documents
        let stats = sqlx::query_as::<_, CollectionStatsRow>(
            "SELECT\n                COUNT(*) as total_documents,\n                AVG(quality_score) as avg_quality_score,\n                AVG(ocr_confidence) as avg_ocr_confidence,\n                SUM(chunk_count) as total_chunks,\n                SUM(CASE WHEN warning_count > 0 THEN 1 ELSE 0 END) as documents_with_warnings\n             FROM documents WHERE collection_id = ?",
        )
        .bind(collection_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to compute collection stats: {}", e))
        })?;

        // Compute average chunk quality
        let avg_chunk_quality = sqlx::query_scalar::<_, Option<f64>>(
            "SELECT AVG(dc.chunk_quality) FROM document_chunks dc\n             INNER JOIN documents d ON dc.doc_id = d.id\n             WHERE d.collection_id = ?",
        )
        .bind(collection_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to compute chunk quality: {}", e)))?;

        // Insert the metrics
        let result = sqlx::query_as::<_, CollectionQualityMetricsEntity>(
            "INSERT INTO collection_quality_metrics\n             (collection_id, avg_quality_score, avg_ocr_confidence, total_documents,\n              documents_with_warnings, total_chunks, avg_chunk_quality)\n             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
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
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to store collection metrics: {}", e))
        })?;

        Ok(result.into())
    }

    /// Record a retrieval gap
    pub async fn record_retrieval_gap(&self, input: &RetrievalGapInput) -> Result<RetrievalGap> {
        let result = sqlx::query_as::<_, RetrievalGapEntity>(
            "INSERT INTO retrieval_gaps (collection_id, query_hash, query_length, result_count,\n                                         max_confidence, avg_confidence, gap_type)\n             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
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
            "SELECT id, collection_id, query_hash, query_length, result_count,\n                    max_confidence, avg_confidence, gap_type, created_at\n             FROM retrieval_gaps WHERE collection_id = ?\n             ORDER BY created_at DESC LIMIT ?",
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
            "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,\n                    quality_score, ocr_confidence, chunk_count, warning_count, created_at\n             FROM documents\n             WHERE collection_id = ? AND (quality_score IS NULL OR quality_score < ?)\n             ORDER BY quality_score ASC NULLS FIRST\n             LIMIT ?",
        )
        .bind(collection_id)
        .bind(threshold)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to get low quality documents: {}", e))
        })?;

        Ok(documents.into_iter().map(|d| d.into()).collect())
    }
}
