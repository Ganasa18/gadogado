use super::LLMClient;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f64,
    #[serde(rename = "topP", skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Deserialize)]
struct GeminiCandidatePart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiModelsResponse {
    models: Option<Vec<GeminiModelInfo>>,
}

#[derive(Deserialize)]
struct GeminiModelInfo {
    name: String,
}

pub struct GeminiClient {
    client: reqwest::Client,
}

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn normalize_model(model: &str) -> String {
        let trimmed = model.trim();

        trimmed.to_string()
    }

    fn api_key(config: &LLMConfig) -> Result<String> {
        config
            .api_key
            .clone()
            .ok_or_else(|| AppError::LLMError("Missing API key for Google provider".to_string()))
    }
}

#[async_trait]
impl LLMClient for GeminiClient {
    async fn generate(&self, config: &LLMConfig, system: &str, user: &str) -> Result<String> {
        let api_key = Self::api_key(config)?;
        let model_id = Self::normalize_model(&config.model);
        let base_url = config.base_url.trim_end_matches('/');
        let url = format!("{}/{}:generateContent?key={}", base_url, model_id, api_key);

        let mut parts = Vec::new();
        if !system.trim().is_empty() {
            parts.push(GeminiPart {
                text: system.to_string(),
            });
        }
        if !user.trim().is_empty() {
            parts.push(GeminiPart {
                text: user.to_string(),
            });
        }

        let body = GeminiRequest {
            contents: vec![GeminiContent { parts, role: None }],
            generation_config: Some(GenerationConfig {
                temperature: config.temperature.unwrap_or(0.7) as f64,
                top_p: None,
                max_output_tokens: config.max_tokens,
            }),
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::LLMError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::LLMError(format!(
                "API error ({}): {}",
                status, text
            )));
        }

        let json: GeminiResponse = response
            .json()
            .await
            .map_err(|e| AppError::LLMError(format!("Failed to parse JSON: {}", e)))?;

        json.candidates
            .get(0)
            .and_then(|candidate| candidate.content.parts.get(0))
            .map(|part| part.text.clone())
            .ok_or_else(|| AppError::LLMError("Invalid response format".to_string()))
    }

    async fn list_models(&self, config: &LLMConfig) -> Result<Vec<String>> {
        let api_key = Self::api_key(config)?;
        let base_url = config.base_url.trim_end_matches('/');
        let url = format!("{}?key={}", base_url, api_key);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::LLMError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::LLMError(format!(
                "API error ({}): {}",
                status, text
            )));
        }

        let json: GeminiModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::LLMError(format!("Failed to parse JSON: {}", e)))?;

        let models = json
            .models
            .unwrap_or_default()
            .into_iter()
            .map(|model| {
                model
                    .name
                    .strip_prefix("models/")
                    .unwrap_or(model.name.as_str())
                    .to_string()
            })
            .collect();

        Ok(models)
    }
}
