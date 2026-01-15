use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{
    RagCollection, RagCollectionInput, RagDocument, RagDocumentChunk, RagDocumentChunkInput,
    RagDocumentInput, RagExcelData, RagExcelDataInput,
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
            "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages, created_at 
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
                "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages, created_at 
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
                "SELECT id, collection_id, file_name, file_path, file_type, language, total_pages, created_at 
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
            "INSERT INTO document_chunks (doc_id, content, page_number, chunk_index, token_count)
             VALUES (?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(input.doc_id)
        .bind(&input.content)
        .bind(input.page_number)
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

    pub async fn get_chunks_with_embeddings(&self, doc_id: i64) -> Result<Vec<(i64, String, Option<Vec<f32>>)>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, chunk_index, token_count, embedding_api
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
            results.push((chunk.id, chunk.content, embedding));
        }

        Ok(results)
    }

    pub async fn get_chunks(&self, doc_id: i64, limit: i64) -> Result<Vec<RagDocumentChunk>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT id, doc_id, content, page_number, chunk_index, token_count 
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

    pub async fn search_chunks_by_collection(
        &self,
        collection_id: i64,
        limit: i64,
    ) -> Result<Vec<(i64, String, Option<Vec<f32>>)>> {
        let chunks = sqlx::query_as::<_, RagDocumentChunkEntity>(
            "SELECT dc.id, dc.doc_id, dc.content, dc.page_number, dc.chunk_index, dc.token_count, dc.embedding_api
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
            results.push((chunk.id, chunk.content, embedding));
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
    chunk_index: i64,
    token_count: Option<i64>,
    embedding_api: Option<Vec<u8>>,
}

impl From<RagDocumentChunkEntity> for RagDocumentChunk {
    fn from(entity: RagDocumentChunkEntity) -> Self {
        Self {
            id: entity.id,
            doc_id: entity.doc_id,
            content: entity.content,
            page_number: entity.page_number,
            chunk_index: entity.chunk_index,
            token_count: entity.token_count,
        }
    }
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
