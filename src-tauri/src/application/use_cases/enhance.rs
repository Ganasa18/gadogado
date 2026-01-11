use crate::domain::error::Result;
use crate::domain::llm_config::LLMConfig;
use crate::domain::prompt::Prompt;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::response::clean_llm_response;
use std::sync::Arc;

pub struct EnhanceUseCase {
    llm_client: Arc<dyn LLMClient + Send + Sync>,
    repository: Arc<SqliteRepository>,
}

impl EnhanceUseCase {
    pub fn new(
        llm_client: Arc<dyn LLMClient + Send + Sync>,
        repository: Arc<SqliteRepository>,
    ) -> Self {
        Self {
            llm_client,
            repository,
        }
    }

    pub async fn execute(
        &self,
        config: &LLMConfig,
        content: String,
        custom_system_prompt: Option<String>,
    ) -> Result<Prompt> {
        let default_prompt = "You are an expert prompt engineer. Improve the following prompt to be more precise, descriptive, and effective for large language models. Ensure clarity and remove ambiguity. Return ONLY the enhanced prompt. Do not include any explanations.";

        let system_prompt = custom_system_prompt.as_deref().unwrap_or(default_prompt);
        let user_prompt = content.clone();

        let raw_result = self
            .llm_client
            .generate(config, system_prompt, &user_prompt)
            .await?;

        let enhanced_text = clean_llm_response(&raw_result);

        let mut prompt = Prompt::new(content, "EN".to_string(), "EN".to_string());
        prompt.result = Some(enhanced_text);

        self.repository.save_prompt(&mut prompt).await?;

        Ok(prompt)
    }
}
