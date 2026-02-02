use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{RagDocument, RagDocumentInput};

use super::entities::RagDocumentEntity;
use super::RagRepository;

impl RagRepository {
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
            "INSERT INTO documents (collection_id, file_name, file_path, file_type, language, total_pages)\n             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
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
            "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,\n                    quality_score, ocr_confidence, chunk_count, warning_count, created_at\n             FROM documents WHERE id = ?",
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

    pub async fn list_documents(
        &self,
        collection_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<RagDocument>> {
        if let Some(collection_id) = collection_id {
            let documents = sqlx::query_as::<_, RagDocumentEntity>(
                "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,\n                        quality_score, ocr_confidence, chunk_count, warning_count, created_at\n                 FROM documents WHERE collection_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(collection_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list documents: {}", e)))?;
            Ok(documents.into_iter().map(|d| d.into()).collect())
        } else {
            let documents = sqlx::query_as::<_, RagDocumentEntity>(
                "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages,\n                        quality_score, ocr_confidence, chunk_count, warning_count, created_at\n                 FROM documents ORDER BY created_at DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to list documents: {}", e)))?;
            Ok(documents.into_iter().map(|d| d.into()).collect())
        }
    }

    /// Get the document type of the first document in a collection.
    /// Returns None if collection is empty, Some(file_type) if documents exist.
    pub async fn get_collection_document_type(
        &self,
        collection_id: i64,
    ) -> Result<Option<String>> {
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT file_type FROM documents WHERE collection_id = ? LIMIT 1",
        )
        .bind(collection_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get collection document type: {}", e)))?;

        Ok(result.map(|(file_type,)| file_type))
    }
}
