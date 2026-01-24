use super::LLMClient;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use async_trait::async_trait;
use serde_json::json;

pub struct OpenRouterClient {
    client: reqwest::Client,
}

impl OpenRouterClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn api_key(config: &LLMConfig) -> Result<String> {
        config
            .api_key
            .clone()
            .ok_or_else(|| AppError::LLMError("Missing API key for OpenRouter".to_string()))
    }
}

#[async_trait]
impl LLMClient for OpenRouterClient {
    async fn generate(&self, config: &LLMConfig, system: &str, user: &str) -> Result<String> {
        let api_key = Self::api_key(config)?;
        let url = if config.base_url.ends_with('/') {
            format!("{}chat/completions", config.base_url)
        } else {
            format!("{}/chat/completions", config.base_url)
        };

        let body = json!({
            "model": config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system
                },
                {
                    "role": "user",
                    "content": user
                }
            ],
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(api_key)
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

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::LLMError(format!("Failed to parse JSON: {}", e)))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::LLMError("Invalid response format".to_string()))
    }

    async fn list_models(&self, config: &LLMConfig) -> Result<Vec<String>> {
        let api_key = Self::api_key(config)?;
        let url = if config.base_url.ends_with('/') {
            format!("{}models", config.base_url)
        } else {
            format!("{}/models", config.base_url)
        };

        let response = self
            .client
            .get(&url)
            .bearer_auth(api_key)
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

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::LLMError(format!("Failed to parse JSON: {}", e)))?;

        let models = json["data"]
            .as_array()
            .ok_or_else(|| {
                AppError::LLMError("Invalid response format: missing data array".to_string())
            })?
            .iter()
            .filter_map(|m| m["id"].as_str())
            .map(|id| id.to_string())
            .collect();

        Ok(models)
    }
}
