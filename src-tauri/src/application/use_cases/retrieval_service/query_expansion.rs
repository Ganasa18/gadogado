use super::RetrievalService;
use std::collections::HashSet;

impl RetrievalService {
    /// Expand the query with synonyms and related terms for better retrieval
    pub(super) fn expand_query(&self, query: &str) -> Vec<String> {
        let mut expansions = Vec::new();
        let lowercase_query = query.to_lowercase();

        // Original query (normalized)
        expansions.push(lowercase_query.clone());

        // Common technical synonyms mapping
        let synonyms: &[(&[&str], &[&str])] = &[
            // Programming terms
            (
                &["function", "func", "fn"],
                &["method", "procedure", "routine"],
            ),
            (&["variable", "var"], &["parameter", "argument", "field"]),
            (&["class"], &["type", "struct", "object"]),
            (
                &["error", "exception"],
                &["bug", "issue", "problem", "failure"],
            ),
            (
                &["create", "make"],
                &["generate", "build", "construct", "initialize"],
            ),
            (
                &["delete", "remove"],
                &["destroy", "drop", "clear", "erase"],
            ),
            (&["update", "modify"], &["change", "edit", "alter", "patch"]),
            (
                &["get", "fetch", "retrieve"],
                &["obtain", "load", "read", "query"],
            ),
            (&["send", "post"], &["submit", "transmit", "push"]),
            (&["array", "list"], &["collection", "vector", "sequence"]),
            (
                &["config", "configuration"],
                &["settings", "options", "preferences"],
            ),
            (&["api"], &["endpoint", "interface", "service"]),
            (&["database", "db"], &["storage", "datastore", "repository"]),
            (
                &["authentication", "auth"],
                &["login", "signin", "authorization"],
            ),
            (&["user"], &["account", "member", "client"]),
            // Document terms
            (&["page"], &["section", "chapter", "part"]),
            (&["summary"], &["overview", "abstract", "synopsis"]),
            (
                &["detail", "details"],
                &["information", "specifics", "particulars"],
            ),
        ];

        // Apply synonym expansion
        for (terms, related) in synonyms {
            for term in *terms {
                if lowercase_query.contains(term) {
                    for synonym in *related {
                        let expanded = lowercase_query.replace(term, synonym);
                        if !expansions.contains(&expanded) {
                            expansions.push(expanded);
                        }
                    }
                    break;
                }
            }
        }

        // Extract key terms (remove stop words)
        let stop_words: HashSet<&str> = [
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with",
            "at", "by", "from", "as", "into", "through", "during", "before", "after", "above",
            "below", "between", "under", "again", "further", "then", "once", "here", "there",
            "when", "where", "why", "how", "all", "each", "few", "more", "most", "other", "some",
            "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very", "just",
            "and", "but", "if", "or", "because", "until", "while", "about", "against", "this",
            "that", "these", "those", "what", "which", "who", "whom", "whose", "i", "me", "my",
            "we", "our", "you", "your", "he", "him", "his", "she", "her", "it", "its", "they",
            "them", "their",
        ]
        .iter()
        .copied()
        .collect();

        let key_terms: Vec<&str> = lowercase_query
            .split_whitespace()
            .filter(|word| {
                word.len() > 2
                    && !stop_words.contains(word.trim_matches(|c: char| !c.is_alphanumeric()))
            })
            .collect();

        // Add individual key terms if query has multiple words
        if key_terms.len() > 1 {
            for term in &key_terms {
                let term_str = term.to_string();
                if !expansions.contains(&term_str) {
                    expansions.push(term_str);
                }
            }
        }

        // Limit expansions to avoid query explosion
        expansions.truncate(5);
        expansions
    }
}
