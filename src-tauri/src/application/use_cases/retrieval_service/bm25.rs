use std::collections::{HashMap, HashSet};

/// BM25 scoring parameters
const BM25_K1: f32 = 1.2; // Term frequency saturation
const BM25_B: f32 = 0.75; // Length normalization

/// Simple BM25 scorer for keyword-based retrieval
pub struct Bm25Scorer {
    /// Document frequencies: term -> number of documents containing term
    doc_frequencies: HashMap<String, usize>,
    /// Total number of documents
    total_docs: usize,
    /// Average document length
    avg_doc_len: f32,
}

impl Bm25Scorer {
    /// Build a BM25 scorer from a collection of documents
    pub fn from_documents(documents: &[&str]) -> Self {
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();
        let mut total_length = 0usize;

        for doc in documents {
            let tokens = Self::tokenize(doc);
            let unique_tokens: HashSet<_> = tokens.iter().collect();

            for token in unique_tokens {
                *doc_frequencies.entry(token.clone()).or_insert(0) += 1;
            }

            total_length += tokens.len();
        }

        let avg_doc_len = if documents.is_empty() {
            1.0
        } else {
            total_length as f32 / documents.len() as f32
        };

        Self {
            doc_frequencies,
            total_docs: documents.len(),
            avg_doc_len,
        }
    }

    /// Score a document against a query
    pub fn score(&self, query: &str, document: &str) -> f32 {
        let query_tokens = Self::tokenize(query);
        let doc_tokens = Self::tokenize(document);
        let doc_len = doc_tokens.len() as f32;

        // Count term frequencies in document
        let mut term_freqs: HashMap<String, usize> = HashMap::new();
        for token in &doc_tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        let mut score = 0.0f32;

        for term in &query_tokens {
            let tf = *term_freqs.get(term).unwrap_or(&0) as f32;
            let df = *self.doc_frequencies.get(term).unwrap_or(&0) as f32;

            if tf > 0.0 && df > 0.0 {
                // IDF component
                let idf = ((self.total_docs as f32 - df + 0.5) / (df + 0.5) + 1.0).ln();

                // TF component with length normalization
                let tf_component = (tf * (BM25_K1 + 1.0))
                    / (tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (doc_len / self.avg_doc_len)));

                score += idf * tf_component;
            }
        }

        score
    }

    /// Tokenize text into lowercase terms
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }
}
