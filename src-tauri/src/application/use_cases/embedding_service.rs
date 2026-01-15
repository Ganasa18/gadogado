use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::llm_config::LLMProvider;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct GeminiEmbeddingRequest {
    content: GeminiEmbeddingContent,
}

#[derive(Debug, Serialize)]
struct GeminiEmbeddingContent {
    parts: Vec<GeminiEmbeddingPart>,
}

#[derive(Debug, Serialize)]
struct GeminiEmbeddingPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbeddingResponse {
    embedding: GeminiEmbeddingResult,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbeddingResult {
    values: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

pub struct EmbeddingService {
    client: Client,
    config: Arc<Mutex<LLMConfig>>,
    local_embedder: Arc<Mutex<Option<TextEmbedding>>>,
}

impl EmbeddingService {
    pub fn new(config: LLMConfig) -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(Mutex::new(config)),
            local_embedder: Arc::new(Mutex::new(None)),
        }
    }

    fn resolve_gemini_embedding_model(model: &str) -> String {
        let trimmed = model.trim().trim_start_matches("models/");
        if trimmed.contains("embedding") {
            trimmed.to_string()
        } else {
            "text-embedding-004".to_string()
        }
    }

    fn resolve_local_embedding_model(model: &str) -> EmbeddingModel {
        match model.trim().to_lowercase().as_str() {
            "all-minilm-l6-v2" => EmbeddingModel::AllMiniLML6V2,
            _ => EmbeddingModel::AllMiniLML6V2,
        }
    }

    pub fn update_config(&self, config: LLMConfig) {
        *self.config.lock().unwrap() = config;
        *self.local_embedder.lock().unwrap() = None;
    }

    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let config = self.config.lock().unwrap().clone();
        match config.provider {
            LLMProvider::Local => self.generate_local_embedding(text, config).await,
            LLMProvider::OpenAI => self.generate_openai_embedding(text, config).await,
            LLMProvider::Google => self.generate_gemini_embedding(text, config).await,
            _ => self.generate_ollama_embedding(text, config).await,
        }
    }

    async fn generate_local_embedding(&self, text: &str, config: LLMConfig) -> Result<Vec<f32>> {
        let model = Self::resolve_local_embedding_model(&config.model);
        let mut guard = self.local_embedder.lock().unwrap();
        if guard.is_none() {
            let mut options = InitOptions::default();
            options.model_name = model;
            let embedder = TextEmbedding::try_new(options)
                .map_err(|e| AppError::Internal(format!("Failed to init local embedder: {}", e)))?;
            *guard = Some(embedder);
        }
        let embedder = guard
            .as_ref()
            .ok_or_else(|| AppError::Internal("Local embedder unavailable".to_string()))?;
        let embeddings = embedder
            .embed(vec![text.to_string()], None)
            .map_err(|e| AppError::Internal(format!("Failed to embed text: {}", e)))?;
        let embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Internal("Empty embedding response".to_string()))?;
        if embedding.is_empty() {
            return Err(AppError::Internal("Empty embedding response".to_string()));
        }
        Ok(embedding)
    }

    async fn generate_openai_embedding(&self, text: &str, config: LLMConfig) -> Result<Vec<f32>> {
        let url = if config.base_url.ends_with("/") {
            format!("{}embeddings", config.base_url)
        } else {
            format!("{}/embeddings", config.base_url)
        };

        let request = OpenAIEmbeddingRequest {
            model: config.model.clone(),
            input: text.to_string(),
        };

        let mut req = self.client.post(&url);
        if let Some(api_key) = &config.api_key {
            req = req.bearer_auth(api_key);
        }

        let response = req
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to call embedding API ({}): {}", url, e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!(
                "Embedding API returned error {} (URL: {}): {}",
                status, url, error_text
            )));
        }

        let embedding_response: OpenAIEmbeddingResponse = response.json().await.map_err(|e| {
            AppError::Internal(format!("Failed to parse embedding response: {}", e))
        })?;

        let embedding = embedding_response
            .data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| AppError::Internal("No embedding data in response".to_string()))?;

        if embedding.is_empty() {
            return Err(AppError::Internal("Empty embedding response".to_string()));
        }

        Ok(embedding)
    }

    async fn generate_gemini_embedding(&self, text: &str, config: LLMConfig) -> Result<Vec<f32>> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| AppError::Internal("Missing API key for Google provider".to_string()))?;
        let mut base_url = config.base_url.trim_end_matches('/').to_string();
        if !base_url.ends_with("/models") {
            base_url = format!("{}/models", base_url);
        }
        let model_id = Self::resolve_gemini_embedding_model(&config.model);
        let url = format!("{}/{}:embedContent?key={}", base_url, model_id, api_key);

        let request = GeminiEmbeddingRequest {
            content: GeminiEmbeddingContent {
                parts: vec![GeminiEmbeddingPart {
                    text: text.to_string(),
                }],
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to call embedding API ({}): {}", url, e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!(
                "Embedding API returned error {} (URL: {}): {}",
                status, url, error_text
            )));
        }

        let embedding_response: GeminiEmbeddingResponse = response.json().await.map_err(|e| {
            AppError::Internal(format!("Failed to parse embedding response: {}", e))
        })?;

        let embedding = embedding_response.embedding.values;
        if embedding.is_empty() {
            return Err(AppError::Internal("Empty embedding response".to_string()));
        }

        Ok(embedding)
    }

    async fn generate_ollama_embedding(&self, text: &str, config: LLMConfig) -> Result<Vec<f32>> {
        let mut url = config.base_url.clone();

        if url.ends_with("/v1") || url.ends_with("/v1/") {
            url = url.trim_end_matches("/v1").to_string();
            url = url.trim_end_matches("/").to_string();
        }

        url = if url.ends_with("/") {
            format!("{}api/embeddings", url)
        } else {
            format!("{}/api/embeddings", url)
        };

        let request = OllamaEmbeddingRequest {
            model: config.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to call embedding API ({}): {}", url, e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!(
                "Embedding API returned error {} (URL: {}): {}",
                status, url, error_text
            )));
        }

        let embedding_response: OllamaEmbeddingResponse = response.json().await.map_err(|e| {
            AppError::Internal(format!("Failed to parse embedding response: {}", e))
        })?;

        let embedding = embedding_response.embedding;

        if embedding.is_empty() {
            return Err(AppError::Internal("Empty embedding response".to_string()));
        }

        Ok(embedding)
    }

    pub async fn generate_embeddings_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        for text in texts {
            let embedding = self.generate_embedding(text).await?;
            embeddings.push(embedding);
        }
        Ok(embeddings)
    }

    pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &val in embedding {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    pub fn bytes_to_embedding(bytes: &[u8]) -> Result<Vec<f32>> {
        if bytes.len() % 4 != 0 {
            return Err(AppError::Internal(
                "Invalid embedding bytes length".to_string(),
            ));
        }

        let mut embedding = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let bytes_array: [u8; 4] = chunk.try_into().unwrap();
            let val = f32::from_le_bytes(bytes_array);
            embedding.push(val);
        }

        Ok(embedding)
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: i64,
    pub content: String,
    pub score: f32,
}

pub struct VectorSearch {
    embedding_service: Arc<EmbeddingService>,
}

impl VectorSearch {
    pub fn new(embedding_service: Arc<EmbeddingService>) -> Self {
        Self { embedding_service }
    }

    pub fn search(
        &self,
        query_embedding: &[f32],
        chunks: &[(i64, String, Option<Vec<f32>>)],
        top_k: usize,
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = chunks
            .iter()
            .filter_map(|(id, content, embedding)| {
                if let Some(embedding) = embedding {
                    let score = EmbeddingService::cosine_similarity(query_embedding, embedding);
                    Some(SearchResult {
                        chunk_id: *id,
                        content: content.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_to_bytes_roundtrip() {
        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let bytes = EmbeddingService::embedding_to_bytes(&embedding);
        let recovered = EmbeddingService::bytes_to_embedding(&bytes).unwrap();
        assert_eq!(embedding, recovered);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        let similarity = EmbeddingService::cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0];
        let similarity = EmbeddingService::cosine_similarity(&a, &c);
        assert!((similarity - 0.0).abs() < 0.001);
    }
}
