use crate::domain::rag_entities::DocumentWarning;
use crate::domain::rag_entities::{
    CollectionQualityMetrics, DbAllowlistProfile, DbConnection, RagCollection, RagDocument,
    RagDocumentChunk, RagExcelData, RetrievalGap,
};

#[derive(sqlx::FromRow)]
pub(super) struct RagCollectionEntity {
    id: i64,
    name: String,
    description: Option<String>,
    kind: String,
    config_json: String,
    created_at: String,
}

impl From<RagCollectionEntity> for RagCollection {
    fn from(entity: RagCollectionEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            description: entity.description,
            kind: entity.kind.into(),
            config_json: entity.config_json,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct RagDocumentEntity {
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
pub(super) struct RagDocumentChunkEntity {
    pub(super) id: i64,
    pub(super) doc_id: i64,
    pub(super) content: String,
    pub(super) page_number: Option<i64>,
    pub(super) page_offset: Option<i64>,
    pub(super) chunk_index: i64,
    pub(super) token_count: Option<i64>,
    pub(super) chunk_quality: Option<f64>,
    pub(super) content_type: Option<String>,
    pub(super) embedding_api: Option<Vec<u8>>,
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

#[derive(sqlx::FromRow)]
pub(super) struct RagExcelDataEntity {
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

#[derive(sqlx::FromRow)]
pub(super) struct DocumentWarningEntity {
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
pub(super) struct CollectionQualityMetricsEntity {
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
pub(super) struct RetrievalGapEntity {
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

#[derive(sqlx::FromRow)]
pub(super) struct DbConnectionEntity {
    id: i64,
    name: String,
    db_type: String,
    host: Option<String>,
    port: Option<i32>,
    database_name: Option<String>,
    username: Option<String>,
    password_ref: Option<String>,
    ssl_mode: String,
    is_enabled: i64,
    created_at: String,
}

impl From<DbConnectionEntity> for DbConnection {
    fn from(entity: DbConnectionEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            db_type: entity.db_type,
            host: entity.host,
            port: entity.port,
            database_name: entity.database_name,
            username: entity.username,
            password_ref: entity.password_ref,
            ssl_mode: entity.ssl_mode,
            is_enabled: entity.is_enabled != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct DbAllowlistProfileEntity {
    id: i64,
    name: String,
    description: Option<String>,
    rules_json: String,
    created_at: String,
}

impl From<DbAllowlistProfileEntity> for DbAllowlistProfile {
    fn from(entity: DbAllowlistProfileEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            description: entity.description,
            rules_json: entity.rules_json,
            created_at: chrono::DateTime::parse_from_rfc3339(&entity.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}
