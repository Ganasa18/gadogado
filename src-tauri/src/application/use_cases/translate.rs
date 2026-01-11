use crate::domain::error::Result;
use crate::domain::llm_config::LLMConfig;
use crate::domain::prompt::Prompt;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::response::clean_llm_response;
use std::sync::Arc;

pub struct TranslateUseCase {
    llm_client: Arc<dyn LLMClient + Send + Sync>,
    repository: Arc<SqliteRepository>,
}

impl TranslateUseCase {
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
        source: String,
        target: String,
    ) -> Result<Prompt> {
        let source_instr = if source == "Auto Detect" {
            "Detect the source language automatically".to_string()
        } else {
            source.clone()
        };

        let system_prompt = format!(
            "You are a professional translator. Translate the following text from {} to {}. Return ONLY the translated text. Do not include any explanations, notes, or quotation marks around the output unless they are in the original text.",
            source_instr, target
        );
        let user_prompt = content.clone();

        let raw_result = self
            .llm_client
            .generate(config, &system_prompt, &user_prompt)
            .await?;

        let translated_text = clean_llm_response(&raw_result);

        let mut prompt = Prompt::new(content, source, target);
        prompt.result = Some(translated_text);

        self.repository.save_prompt(&mut prompt).await?;

        Ok(prompt)
    }
}
