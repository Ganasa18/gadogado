use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub content: String,
    pub source_type: String,
    pub source_id: i64,
    pub score: Option<f32>,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub doc_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryAnalysis {
    pub query_type: QueryType,
    pub numeric_queries: Vec<NumericQuery>,
    pub structured: StructuredQueryHints,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryType {
    TextOnly,
    NumericOnly,
    Hybrid,
    /// Aggregate/list/count/filter style queries over structured_rows
    Structured,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructuredQueryHints {
    pub wants_aggregate: bool,
    pub wants_count: bool,
    pub wants_sources: bool,
    pub wants_titles: bool,
    pub category: Option<String>,
    pub source: Option<String>,
    pub keyword: Option<String>,
    pub requested_limit: Option<usize>,
}

impl StructuredQueryHints {
    pub(super) fn empty() -> Self {
        Self {
            wants_aggregate: false,
            wants_count: false,
            wants_sources: false,
            wants_titles: false,
            category: None,
            source: None,
            keyword: None,
            requested_limit: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NumericQuery {
    pub column: String,
    pub operator: String,
    pub value: String,
}
