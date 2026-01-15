use crate::application::use_cases::embedding_service::{EmbeddingService, VectorSearch};
use crate::domain::error::Result;
// use crate::domain::rag_entities::{RagDocumentChunk, RagExcelData};
use crate::infrastructure::db::rag::repository::RagRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub content: String,
    pub source_type: String,
    pub source_id: i64,
    pub score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryAnalysis {
    pub query_type: QueryType,
    pub numeric_queries: Vec<NumericQuery>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryType {
    TextOnly,
    NumericOnly,
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NumericQuery {
    pub column: String,
    pub operator: String,
    pub value: String,
}

pub struct RetrievalService {
    rag_repository: Arc<RagRepository>,
    embedding_service: Arc<EmbeddingService>,
    vector_search: VectorSearch,
}

impl RetrievalService {
    pub fn new(
        rag_repository: Arc<RagRepository>,
        embedding_service: Arc<EmbeddingService>,
    ) -> Self {
        let vector_search = VectorSearch::new(embedding_service.clone());
        Self {
            rag_repository,
            embedding_service,
            vector_search,
        }
    }

    pub async fn query(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let analysis = self.analyze_query(query_text);
        let mut results = Vec::new();

        match analysis.query_type {
            QueryType::NumericOnly => {
                let excel_results = self
                    .retrieve_excel_data(collection_id, &analysis.numeric_queries, top_k)
                    .await?;
                results.extend(excel_results);
            }
            QueryType::TextOnly => {
                let text_results = self
                    .retrieve_text_chunks(collection_id, query_text, top_k)
                    .await?;
                results.extend(text_results);
            }
            QueryType::Hybrid => {
                let excel_results = self
                    .retrieve_excel_data(collection_id, &analysis.numeric_queries, top_k / 2)
                    .await?;
                let text_results = self
                    .retrieve_text_chunks(collection_id, query_text, top_k / 2)
                    .await?;
                results.extend(excel_results);
                results.extend(text_results);
            }
        }

        Ok(results)
    }

    fn analyze_query(&self, query: &str) -> QueryAnalysis {
        let lowercase_query = query.to_lowercase();
        let mut numeric_queries = Vec::new();
        let mut has_numeric = false;
        let mut has_text = false;

        let keywords = ["val_a", "val_b", "column", "field", "value", "numeric"];
        for keyword in &keywords {
            if lowercase_query.contains(keyword) {
                has_numeric = true;
                break;
            }
        }

        if lowercase_query.contains("=") || lowercase_query.contains("equals") {
            if let Some(part) = lowercase_query.split('=').nth(1) {
                let value = part.trim().to_string();
                numeric_queries.push(NumericQuery {
                    column: "val_a".to_string(),
                    operator: "=".to_string(),
                    value,
                });
                has_numeric = true;
            }
        }

        let text_keywords = ["what", "how", "why", "explain", "describe", "summarize"];
        for keyword in &text_keywords {
            if lowercase_query.contains(keyword) {
                has_text = true;
                break;
            }
        }

        if !has_text && !has_numeric {
            has_text = true;
        }

        let query_type = if has_numeric && has_text {
            QueryType::Hybrid
        } else if has_numeric {
            QueryType::NumericOnly
        } else {
            QueryType::TextOnly
        };

        QueryAnalysis {
            query_type,
            numeric_queries,
        }
    }

    async fn retrieve_text_chunks(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let query_embedding = self
            .embedding_service
            .generate_embedding(query_text)
            .await?;

        let chunks = self
            .rag_repository
            .search_chunks_by_collection(collection_id, 1000)
            .await?;

        let search_results = self.vector_search.search(&query_embedding, &chunks, top_k);

        let mut results = Vec::new();
        for result in search_results {
            results.push(QueryResult {
                content: result.content,
                source_type: "text_chunk".to_string(),
                source_id: result.chunk_id,
                score: Some(result.score),
            });
        }

        Ok(results)
    }

    async fn retrieve_excel_data(
        &self,
        collection_id: i64,
        queries: &[NumericQuery],
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let mut column_a = None;
        let mut column_b = None;

        for query in queries {
            if query.column == "val_a" && query.operator == "=" {
                column_a = Some(query.value.as_str());
            } else if query.column == "val_b" && query.operator == "=" {
                column_b = Some(query.value.as_str());
            }
        }

        let excel_data = self
            .rag_repository
            .search_excel_by_collection_with_filter(collection_id, column_a, column_b, top_k as i64)
            .await?;

        let mut results = Vec::new();
        for row in excel_data {
            let content = format!(
                "Row {}: val_a={}, val_b={}, val_c={}",
                row.row_index,
                row.val_a.unwrap_or_else(|| "null".to_string()),
                row.val_b.unwrap_or_else(|| "null".to_string()),
                row.val_c
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string())
            );
            results.push(QueryResult {
                content,
                source_type: "excel_data".to_string(),
                source_id: row.id,
                score: None,
            });
        }

        Ok(results)
    }
}
