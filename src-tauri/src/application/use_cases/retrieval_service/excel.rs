use super::{NumericQuery, QueryResult, RetrievalService};
use crate::domain::error::Result;

impl RetrievalService {
    pub(super) async fn retrieve_excel_data(
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
                page_number: None,
                page_offset: None,
                doc_name: None,
            });
        }

        Ok(results)
    }
}
