use crate::application::use_cases::rag_ingestion::RagIngestionUseCase;
use crate::application::use_cases::retrieval_service::{QueryResult, RetrievalService};
use crate::domain::error::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCase {
    pub collection_id: i64,
    pub query: String,
    pub expected_keywords: Vec<String>,
    pub document_id: Option<i64>,
    pub top_k: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOptions {
    pub top_k: usize,
    pub use_cache: bool,
    pub optimized: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            top_k: 5,
            use_cache: true,
            optimized: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub query: String,
    pub result_count: usize,
    pub retrieval_precision: f32,
    pub answer_relevance: f32,
    pub chunking_quality: f32,
    pub extraction_accuracy: f32,
    pub latency_ms: u64,
    pub cache_hit: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub total_cases: usize,
    pub avg_retrieval_precision: f32,
    pub avg_answer_relevance: f32,
    pub avg_chunking_quality: f32,
    pub avg_extraction_accuracy: f32,
    pub avg_latency_ms: f32,
    pub results: Vec<ValidationResult>,
}

pub struct RagValidationSuite;

impl RagValidationSuite {
    pub async fn run(
        retrieval_service: &RetrievalService,
        ingestion: &RagIngestionUseCase,
        cases: &[ValidationCase],
        options: &ValidationOptions,
    ) -> Result<ValidationReport> {
        let mut results = Vec::new();
        let mut totals = Totals::default();

        for case in cases {
            let top_k = case.top_k.unwrap_or(options.top_k);
            let start = Instant::now();

            let (mut retrieved, cache_hit) = if options.use_cache {
                retrieval_service
                    .query_cached(case.collection_id, &case.query, top_k)
                    .await?
            } else {
                (
                    retrieval_service
                        .query(case.collection_id, &case.query, top_k)
                        .await?,
                    false,
                )
            };

            if options.optimized {
                retrieved = retrieval_service.optimize_context(retrieved);
                retrieved.truncate(top_k);
            }

            let latency_ms = start.elapsed().as_millis() as u64;
            let mut issues = Vec::new();

            if retrieved.is_empty() {
                issues.push("no_results".to_string());
            }

            let (retrieval_precision, answer_relevance) =
                Self::score_expected_keywords(&retrieved, &case.expected_keywords, &mut issues);
            let chunking_quality = Self::average_chunk_quality(&retrieved);

            let extraction_accuracy = if let Some(document_id) = case.document_id {
                ingestion
                    .analyze_document_quality(document_id)
                    .await
                    .map(|analysis| analysis.avg_chunk_quality)?
            } else {
                chunking_quality
            };

            let result = ValidationResult {
                query: case.query.clone(),
                result_count: retrieved.len(),
                retrieval_precision,
                answer_relevance,
                chunking_quality,
                extraction_accuracy,
                latency_ms,
                cache_hit,
                issues,
            };

            totals.add(&result);
            results.push(result);
        }

        Ok(totals.build_report(results))
    }

    fn score_expected_keywords(
        results: &[QueryResult],
        expected_keywords: &[String],
        issues: &mut Vec<String>,
    ) -> (f32, f32) {
        if expected_keywords.is_empty() {
            issues.push("no_expected_keywords".to_string());
            return (0.0, 0.0);
        }

        let keywords: Vec<String> = expected_keywords.iter().map(|k| k.to_lowercase()).collect();
        let mut matched_results = 0usize;
        let mut matched_keywords = 0usize;

        for keyword in &keywords {
            if results
                .iter()
                .any(|r| Self::contains_keyword(&r.content, keyword))
            {
                matched_keywords += 1;
            }
        }

        for result in results {
            if keywords
                .iter()
                .any(|keyword| Self::contains_keyword(&result.content, keyword))
            {
                matched_results += 1;
            }
        }

        if matched_keywords == 0 {
            issues.push("no_keyword_match".to_string());
        }

        let retrieval_precision = if results.is_empty() {
            0.0
        } else {
            matched_results as f32 / results.len() as f32
        };
        let answer_relevance = matched_keywords as f32 / keywords.len() as f32;

        (retrieval_precision, answer_relevance)
    }

    fn contains_keyword(content: &str, keyword: &str) -> bool {
        content.to_lowercase().contains(keyword)
    }

    fn average_chunk_quality(results: &[QueryResult]) -> f32 {
        if results.is_empty() {
            return 0.0;
        }

        let total = results
            .iter()
            .map(|result| Self::estimate_chunk_quality(&result.content))
            .sum::<f32>();

        total / results.len() as f32
    }

    fn estimate_chunk_quality(content: &str) -> f32 {
        let mut score = 0.0f32;
        let len = content.len();

        if len >= 100 && len <= 500 {
            score += 0.3;
        } else if len >= 50 && len <= 800 {
            score += 0.2;
        } else if len < 50 {
            score += 0.05;
        } else {
            score += 0.1;
        }

        let has_alphanumeric = content.chars().any(|c| c.is_alphanumeric());
        let alpha_ratio =
            content.chars().filter(|c| c.is_alphabetic()).count() as f32 / len.max(1) as f32;
        let has_sentences = content.contains('.') || content.contains('!') || content.contains('?');
        let has_capital = content.chars().any(|c| c.is_uppercase());

        if has_alphanumeric {
            score += 0.2;
        }
        if alpha_ratio > 0.5 {
            score += 0.2;
        }
        if has_sentences {
            score += 0.15;
        }
        if has_capital {
            score += 0.15;
        }

        score.min(1.0)
    }
}

#[derive(Default)]
struct Totals {
    count: usize,
    sum_retrieval_precision: f32,
    sum_answer_relevance: f32,
    sum_chunking_quality: f32,
    sum_extraction_accuracy: f32,
    sum_latency_ms: f32,
}

impl Totals {
    fn add(&mut self, result: &ValidationResult) {
        self.count += 1;
        self.sum_retrieval_precision += result.retrieval_precision;
        self.sum_answer_relevance += result.answer_relevance;
        self.sum_chunking_quality += result.chunking_quality;
        self.sum_extraction_accuracy += result.extraction_accuracy;
        self.sum_latency_ms += result.latency_ms as f32;
    }

    fn build_report(self, results: Vec<ValidationResult>) -> ValidationReport {
        let count = self.count.max(1) as f32;
        ValidationReport {
            total_cases: results.len(),
            avg_retrieval_precision: self.sum_retrieval_precision / count,
            avg_answer_relevance: self.sum_answer_relevance / count,
            avg_chunking_quality: self.sum_chunking_quality / count,
            avg_extraction_accuracy: self.sum_extraction_accuracy / count,
            avg_latency_ms: self.sum_latency_ms / count,
            results,
        }
    }
}
