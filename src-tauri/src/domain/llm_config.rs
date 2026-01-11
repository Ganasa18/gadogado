use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LLMProvider {
    Local,
    OpenAI,
    Anthropic,
    Google,
    #[serde(rename = "DLL")]
    Dll,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::Local,
            base_url: "http://localhost:1234/v1".to_string(),
            model: "local-model".to_string(),
            api_key: None,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        }
    }
}
