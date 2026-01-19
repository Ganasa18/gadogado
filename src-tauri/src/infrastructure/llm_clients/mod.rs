pub mod gemini;
pub mod openai;

use crate::domain::error::Result;
use crate::domain::llm_config::LLMConfig;
use crate::domain::llm_config::LLMProvider;
use async_trait::async_trait;
use gemini::GeminiClient;
use openai::OpenAIClient;

#[async_trait]
pub trait LLMClient {
    async fn generate(&self, config: &LLMConfig, system: &str, user: &str) -> Result<String>;
    async fn list_models(&self, config: &LLMConfig) -> Result<Vec<String>>;
}

pub struct RouterClient {
    openai: OpenAIClient,
    gemini: GeminiClient,
}

impl RouterClient {
    pub fn new() -> Self {
        Self {
            openai: OpenAIClient::new(),
            gemini: GeminiClient::new(),
        }
    }
}

#[async_trait]
impl LLMClient for RouterClient {
    async fn generate(&self, config: &LLMConfig, system: &str, user: &str) -> Result<String> {
        match config.provider {
            LLMProvider::Gemini => self.gemini.generate(config, system, user).await,
            _ => self.openai.generate(config, system, user).await,
        }
    }

    async fn list_models(&self, config: &LLMConfig) -> Result<Vec<String>> {
        match config.provider {
            LLMProvider::Gemini => self.gemini.list_models(config).await,
            _ => self.openai.list_models(config).await,
        }
    }
}
