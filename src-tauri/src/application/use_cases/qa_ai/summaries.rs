use super::{QaAiUseCase, PROMPT_VERSION};
use crate::application::use_cases::qa_ai::event_text::{
    build_chunked_event_text, build_input_summary,
};
use crate::application::use_cases::qa_ai::hashing::{hash_input, normalize_language};
use crate::application::use_cases::qa_ai::llm_output::extract_json_payload;
use crate::application::use_cases::qa_ai::prompts::{
    build_summary_system_prompt, build_summary_user_prompt,
};
use crate::application::use_cases::qa_ai::types::SummaryOutput;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::qa_checkpoint::{QaCheckpointSummary, QaLlmRun};
use crate::infrastructure::response::clean_llm_response;
use uuid::Uuid;

impl QaAiUseCase {
    pub async fn generate_checkpoint_summary(
        &self,
        session_id: &str,
        checkpoint_id: &str,
        config: &LLMConfig,
        output_language: &str,
    ) -> Result<QaCheckpointSummary> {
        let session = self.session_repository.get_session(session_id).await?;
        let checkpoint = self
            .checkpoint_repository
            .get_checkpoint(checkpoint_id)
            .await?;
        if checkpoint.session_id != session.id {
            return Err(AppError::ValidationError(
                "Checkpoint does not belong to session.".to_string(),
            ));
        }

        let events = self
            .event_repository
            .list_events_range(
                session_id,
                checkpoint.start_event_seq,
                checkpoint.end_event_seq,
            )
            .await?;
        if events.is_empty() {
            return Err(AppError::ValidationError("Checkpoint has no events.".to_string()));
        }

        let chunked = build_chunked_event_text(&events);
        let input_summary = build_input_summary(&session, &checkpoint, chunked.len());
        let language = normalize_language(output_language);
        let system_prompt = build_summary_system_prompt(&language);
        let user_prompt = build_summary_user_prompt(&session, &checkpoint, &chunked, &language);

        let raw_output = self
            .llm_client
            .generate(config, &system_prompt, &user_prompt)
            .await?;
        let cleaned = clean_llm_response(&raw_output);
        let normalized = extract_json_payload(&cleaned);

        let parsed = serde_json::from_str::<SummaryOutput>(&normalized).ok();
        let summary_text = parsed
            .as_ref()
            .map(|output| output.summary_text.clone())
            .unwrap_or_else(|| cleaned.clone());
        let entities_json = parsed
            .as_ref()
            .and_then(|output| output.entities.as_ref())
            .and_then(|items| serde_json::to_string(items).ok());
        let risks_json = parsed
            .as_ref()
            .and_then(|output| output.risks.as_ref())
            .and_then(|items| serde_json::to_string(items).ok());

        let output_json = parsed
            .as_ref()
            .and_then(|output| serde_json::to_string(output).ok())
            .unwrap_or_else(|| normalized.clone());

        let created_at = chrono::Utc::now().timestamp_millis();
        let summary_id = Uuid::new_v4().to_string();
        let summary = self
            .checkpoint_repository
            .insert_checkpoint_summary(
                summary_id,
                checkpoint_id,
                summary_text,
                entities_json,
                risks_json,
                created_at,
            )
            .await?;

        let run = QaLlmRun {
            id: Uuid::new_v4().to_string(),
            scope: "checkpoint_summary".to_string(),
            scope_id: checkpoint_id.to_string(),
            model: config.model.clone(),
            prompt_version: Some(PROMPT_VERSION.to_string()),
            input_digest: Some(hash_input(&input_summary, &config.model)),
            input_summary: Some(input_summary),
            output_json,
            created_at,
        };
        self.checkpoint_repository.insert_llm_run(&run).await?;

        Ok(summary)
    }
}
