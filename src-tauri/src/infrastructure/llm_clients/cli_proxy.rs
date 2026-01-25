use super::LLMClient;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;

pub struct CliProxyClient {
    client: reqwest::Client,
}

impl CliProxyClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for CliProxyClient {
    async fn generate(&self, config: &LLMConfig, system: &str, user: &str) -> Result<String> {
        // CLI Proxy uses /chat/completions endpoint
        let url = if config.base_url.ends_with("/") {
            format!("{}chat/completions", config.base_url)
        } else {
            format!("{}/chat/completions", config.base_url)
        };

        let mut request = self.client.post(&url);

        // CLI Proxy uses raw Authorization header (not Bearer token)
        if let Some(api_key) = &config.api_key {
            request = request.header("Authorization", api_key);
        }

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

        let response = request
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
        let url = if config.base_url.ends_with("/") {
            format!("{}models", config.base_url)
        } else {
            format!("{}/models", config.base_url)
        };

        let mut request = self.client.get(&url);

        // CLI Proxy uses raw Authorization header (not Bearer token)
        if let Some(api_key) = &config.api_key {
            request = request.header("Authorization", api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::LLMError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::LLMError(format!("API error ({})", status)));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::LLMError(format!("Failed to parse JSON: {}", e)))?;

        let models: Vec<String> = json["data"]
            .as_array()
            .ok_or_else(|| {
                AppError::LLMError("Invalid response format: missing data array".to_string())
            })?
            .iter()
            .filter_map(|m| m["id"].as_str())
            .map(|id| id.to_string())
            .collect();

        // Deduplicate model IDs (CLI Proxy may return duplicates)
        let unique_models: Vec<String> = models
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        Ok(unique_models)
    }
}
