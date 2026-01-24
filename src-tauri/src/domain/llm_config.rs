use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    Local,
    OpenAI,
    #[serde(alias = "google", alias = "Google")]
    Gemini,
    #[serde(alias = "ollama")]
    Ollama,
    #[serde(alias = "llama_cpp")]
    LlamaCpp,
    #[serde(alias = "openrouter")]
    OpenRouter,
    #[serde(alias = "dll", rename = "dll")]
    Dll,
}

impl fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMProvider::Local => write!(f, "local"),
            LLMProvider::OpenAI => write!(f, "openai"),
            LLMProvider::Gemini => write!(f, "gemini"),
            LLMProvider::Ollama => write!(f, "ollama"),
            LLMProvider::LlamaCpp => write!(f, "llama_cpp"),
            LLMProvider::OpenRouter => write!(f, "openrouter"),
            LLMProvider::Dll => write!(f, "dll"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    #[serde(alias = "baseUrl")]
    pub base_url: String,
    pub model: String,
    #[serde(alias = "apiKey")]
    pub api_key: Option<String>,
    #[serde(alias = "maxTokens")]
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// Chat message with role and content
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::Local,
            base_url: "http://localhost:11434".to_string(),
            model: "nomic-embed-text".to_string(),
            api_key: None,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        }
    }
}
