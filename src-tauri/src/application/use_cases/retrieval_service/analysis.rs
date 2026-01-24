use super::{NumericQuery, QueryAnalysis, QueryType, RetrievalService, StructuredQueryHints};
use sha2::{Digest, Sha256};

impl RetrievalService {
    pub(super) fn hash_query(query: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let digest = hasher.finalize();
        let hex = hex::encode(digest);
        hex.chars().take(16).collect()
    }

    pub(super) fn analyze_query(&self, query: &str) -> QueryAnalysis {
        let lowercase_query = query.to_lowercase();
        let mut numeric_queries = Vec::new();
        let mut has_numeric = false;
        let mut has_text = false;

        // Aggregate/list/count queries should be routed to structured_rows.
        let structured = self.analyze_structured_query(&lowercase_query);

        let keywords = ["val_a", "val_b", "column", "field", "value", "numeric"];
        for keyword in &keywords {
            if lowercase_query.contains(keyword) {
                has_numeric = true;
                break;
            }
        }

        if lowercase_query.contains('=') || lowercase_query.contains("equals") {
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

        let query_type = if structured.wants_aggregate {
            QueryType::Structured
        } else if has_numeric && has_text {
            QueryType::Hybrid
        } else if has_numeric {
            QueryType::NumericOnly
        } else {
            QueryType::TextOnly
        };

        QueryAnalysis {
            query_type,
            numeric_queries,
            structured,
        }
    }

    fn analyze_structured_query(&self, lowercase_query: &str) -> StructuredQueryHints {
        // Heuristic intent router based on keywords (matches IMPLEMENTATION_PROGRESS_V2.md).
        let agg_keywords = [
            "all",
            "list",
            "collect",
            "count",
            "total",
            "daftar",
            "kumpulkan",
            "semua",
            "berapa",
        ];

        // Many users will ask for structured data without explicit aggregate keywords
        // (e.g. "cari source ai" / "kategori ai"). If the query contains explicit
        // structured filters, treat it as a structured/aggregate intent.
        let has_explicit_filters = [
            "category:",
            "kategori:",
            "source:",
            "sumber:",
            "kata kunci",
            "keyword",
        ]
        .iter()
        .any(|k| lowercase_query.contains(k));

        let has_structured_markers = [
            "category", "kategori", "source", "sumber", "baris", "row", "data",
        ]
        .iter()
        .any(|k| lowercase_query.contains(k));

        let has_ai_token = lowercase_query
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|t| t == "ai");

        let initial_wants_aggregate = agg_keywords.iter().any(|k| lowercase_query.contains(k))
            || has_explicit_filters
            || (has_structured_markers && has_ai_token);

        let wants_count = ["count", "total", "berapa"]
            .iter()
            .any(|k| lowercase_query.contains(k));
        let wants_sources = ["source", "sources", "sumber"]
            .iter()
            .any(|k| lowercase_query.contains(k));
        let mut wants_titles = ["title", "titles", "judul"]
            .iter()
            .any(|k| lowercase_query.contains(k));

        let mut category = None;
        let mut source = None;
        let mut keyword = None;
        let mut requested_limit: Option<usize> = None;

        // category:xxx / kategori:xxx
        if let Some((_, val)) = lowercase_query.split_once("category") {
            if let Some(v) = val.split(|c: char| c == ':' || c == '=').nth(1) {
                let v = v.trim().split_whitespace().next().unwrap_or("");
                if !v.is_empty() {
                    category = Some(v.to_string());
                }
            }
        }
        if category.is_none() {
            if let Some((_, val)) = lowercase_query.split_once("kategori") {
                if let Some(v) = val.split(|c: char| c == ':' || c == '=').nth(1) {
                    let v = v.trim().split_whitespace().next().unwrap_or("");
                    if !v.is_empty() {
                        category = Some(v.to_string());
                    }
                }
            }
        }

        // source:xxx / sumber:xxx
        if let Some((_, val)) = lowercase_query.split_once("source") {
            if let Some(v) = val.split(|c: char| c == ':' || c == '=').nth(1) {
                let v = v.trim().split_whitespace().next().unwrap_or("");
                if !v.is_empty() {
                    source = Some(v.to_string());
                }
            }
        }
        if source.is_none() {
            if let Some((_, val)) = lowercase_query.split_once("sumber") {
                if let Some(v) = val.split(|c: char| c == ':' || c == '=').nth(1) {
                    let v = v.trim().split_whitespace().next().unwrap_or("");
                    if !v.is_empty() {
                        source = Some(v.to_string());
                    }
                }
            }
        }

        // Special-case: "AI" category is common in the spec.
        // If the user is explicitly doing keyword search ("kata kunci"), don't force category.
        if category.is_none() && keyword.is_none() {
            let tokens: Vec<&str> = lowercase_query
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|t| !t.is_empty())
                .collect();
            if tokens.iter().any(|t| *t == "ai") {
                category = Some("ai".to_string());
            }
        }

        // keyword extraction: "kata kunci X" / "keyword X"
        if let Some((_, tail)) = lowercase_query.split_once("kata kunci") {
            let v = tail.trim().split_whitespace().next().unwrap_or("");
            if !v.is_empty() {
                keyword = Some(v.to_string());
            }
        }
        if keyword.is_none() {
            if let Some((_, tail)) = lowercase_query.split_once("keyword") {
                let v = tail.trim().split_whitespace().next().unwrap_or("");
                if !v.is_empty() {
                    keyword = Some(v.to_string());
                }
            }
        }

        // requested limit: "1 row" / "1 data" / "1 baris"
        // (we keep it simple: scan for a number followed by one of these tokens)
        let tokens: Vec<&str> = lowercase_query.split_whitespace().collect();
        for i in 0..tokens.len() {
            if let Ok(n) = tokens[i].parse::<usize>() {
                let next = tokens.get(i + 1).copied().unwrap_or("");
                if ["row", "rows", "data", "baris"].contains(&next) {
                    if n > 0 {
                        requested_limit = Some(n);
                        break;
                    }
                }
            }
        }

        // If we extracted any structured filters, treat it as structured intent even if the user
        // didn't use explicit aggregate keywords.
        let wants_aggregate = initial_wants_aggregate
            || category.is_some()
            || source.is_some()
            || keyword.is_some()
            || wants_count
            || wants_sources
            || wants_titles;

        if !wants_aggregate {
            return StructuredQueryHints::empty();
        }

        // Default for aggregate queries: if user didn't ask for count/sources/titles explicitly,
        // we still return rows (titles/content snippets) instead of falling back to vector QA.
        if !wants_count && !wants_sources && !wants_titles {
            wants_titles = true;
        }

        StructuredQueryHints {
            wants_aggregate,
            wants_count,
            wants_sources,
            wants_titles,
            category,
            source,
            keyword,
            requested_limit,
        }
    }
}
