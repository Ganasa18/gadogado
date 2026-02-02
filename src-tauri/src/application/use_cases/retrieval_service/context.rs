use super::{QueryResult, RetrievalService};
use std::cmp::Ordering;
use std::collections::HashSet;

impl RetrievalService {
    /// Optimize retrieved results by removing duplicates and enriching metadata
    pub fn optimize_context(&self, results: Vec<QueryResult>) -> Vec<QueryResult> {
        let mut optimized = self.remove_duplicates(results);
        optimized = self.merge_adjacent_chunks(optimized);
        self.enrich_metadata(optimized)
    }

    /// Remove duplicate or near-duplicate results based on content similarity
    fn remove_duplicates(&self, results: Vec<QueryResult>) -> Vec<QueryResult> {
        let mut unique_results: Vec<QueryResult> = Vec::new();
        let similarity_threshold = 0.85; // Content overlap threshold

        for result in results {
            let is_duplicate = unique_results.iter().any(|existing| {
                self.content_similarity(&existing.content, &result.content) > similarity_threshold
            });

            if !is_duplicate {
                unique_results.push(result);
            }
        }

        unique_results
    }

    /// Calculate Jaccard similarity between two pieces of text
    fn content_similarity(&self, text1: &str, text2: &str) -> f32 {
        let text1_lower = text1.to_lowercase();
        let text2_lower = text2.to_lowercase();

        let words1: HashSet<&str> = text1_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();
        let words2: HashSet<&str> = text2_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        intersection as f32 / union as f32
    }

    /// Merge adjacent chunks from the same document for better context
    fn merge_adjacent_chunks(&self, mut results: Vec<QueryResult>) -> Vec<QueryResult> {
        if results.len() < 2 {
            return results;
        }

        // Sort by document and page/offset for adjacency detection
        results.sort_by(|a, b| {
            let doc_cmp = a.doc_name.cmp(&b.doc_name);
            if doc_cmp != Ordering::Equal {
                return doc_cmp;
            }
            let page_cmp = a.page_number.cmp(&b.page_number);
            if page_cmp != Ordering::Equal {
                return page_cmp;
            }
            a.page_offset.cmp(&b.page_offset)
        });

        let mut merged: Vec<QueryResult> = Vec::new();
        let mut i = 0;

        while i < results.len() {
            let mut current = results[i].clone();

            // Look for adjacent chunks from same document
            while i + 1 < results.len() {
                let next = &results[i + 1];

                // Check if adjacent (same doc, consecutive pages or close offsets)
                let is_adjacent = current.doc_name == next.doc_name
                    && current.page_number == next.page_number
                    && match (current.page_offset, next.page_offset) {
                        (Some(curr_off), Some(next_off)) => {
                            // Adjacent if within 600 chars (chunk size + overlap)
                            (next_off - curr_off).abs() < 600
                        }
                        _ => false,
                    };

                if is_adjacent {
                    // Merge content
                    current.content = format!("{}\n\n{}", current.content, next.content);
                    // Take max score
                    current.score = match (current.score, next.score) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (a, b) => a.or(b),
                    };
                    i += 1;
                } else {
                    break;
                }
            }

            merged.push(current);
            i += 1;
        }

        // Re-sort by score
        merged.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
        });

        merged
    }

    /// Enrich metadata for better citation and context
    fn enrich_metadata(&self, mut results: Vec<QueryResult>) -> Vec<QueryResult> {
        for result in &mut results {
            // Add content preview if content is very long
            if result.content.len() > 500 {
                // Keep full content but could add a preview field in the future
            }

            // Ensure source_type is descriptive
            if result.source_type == "text_chunk" {
                if let Some(ref doc_name) = result.doc_name {
                    if doc_name.ends_with(".pdf") {
                        result.source_type = "pdf_chunk".to_string();
                    } else if doc_name.ends_with(".docx") {
                        result.source_type = "docx_chunk".to_string();
                    } else if doc_name.ends_with(".md") {
                        result.source_type = "markdown_chunk".to_string();
                    }
                }
            }
        }

        results
    }
}
