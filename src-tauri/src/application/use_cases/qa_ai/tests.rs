use super::{QaAiUseCase, PROMPT_VERSION};
use crate::application::use_cases::qa_ai::event_text::{
    build_chunked_event_text, build_input_summary, preview_text,
};
use crate::application::use_cases::qa_ai::hashing::{hash_input, normalize_language};
use crate::application::use_cases::qa_ai::llm_output::extract_json_payload;
use crate::application::use_cases::qa_ai::prompts::{build_test_system_prompt, build_test_user_prompt};
use crate::application::use_cases::qa_ai::types::TestCaseOutput;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::qa_checkpoint::{QaLlmRun, QaTestCase};
use crate::infrastructure::response::clean_llm_response;
use uuid::Uuid;

impl QaAiUseCase {
    pub async fn generate_test_cases(
        &self,
        session_id: &str,
        checkpoint_id: &str,
        config: &LLMConfig,
        output_language: &str,
    ) -> Result<Vec<QaTestCase>> {
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

        let summary = self
            .checkpoint_repository
            .get_checkpoint_summary(checkpoint_id)
            .await?;
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
        let existing_cases = self
            .checkpoint_repository
            .list_test_cases_for_checkpoint(checkpoint_id)
            .await?;

        let input_summary = build_input_summary(&session, &checkpoint, chunked.len());
        let language = normalize_language(output_language);
        let system_prompt = build_test_system_prompt(&language);
        let user_prompt = build_test_user_prompt(
            &session,
            &checkpoint,
            summary.as_ref(),
            &chunked,
            &existing_cases,
            &language,
        );

        let raw_output = self
            .llm_client
            .generate(config, &system_prompt, &user_prompt)
            .await?;
        let cleaned = clean_llm_response(&raw_output);
        let normalized = extract_json_payload(&cleaned);
        let parsed = serde_json::from_str::<TestCaseOutput>(&normalized).map_err(|err| {
            let snippet = preview_text(&normalized, 600);
            AppError::Internal(format!(
                "Failed to parse LLM test case output: {} | output_snippet={}",
                err, snippet
            ))
        })?;

        let output_json = serde_json::to_string(&parsed).unwrap_or_else(|_| normalized.clone());
        let created_at = chrono::Utc::now().timestamp_millis();

        let mut stored = Vec::new();
        stored.extend(
            self.store_test_cases(
                checkpoint_id,
                &session,
                &parsed.negative_cases,
                "negative",
                created_at,
            )
            .await?,
        );
        stored.extend(
            self.store_test_cases(
                checkpoint_id,
                &session,
                &parsed.edge_cases,
                "edge",
                created_at,
            )
            .await?,
        );
        stored.extend(
            self.store_test_cases(
                checkpoint_id,
                &session,
                &parsed.exploratory_charters,
                "exploratory",
                created_at,
            )
            .await?,
        );
        if session.session_type == "api" {
            stored.extend(
                self.store_test_cases(
                    checkpoint_id,
                    &session,
                    &parsed.api_gap_checks,
                    "api_gap",
                    created_at,
                )
                .await?,
            );
        }

        let run = QaLlmRun {
            id: Uuid::new_v4().to_string(),
            scope: "test_cases".to_string(),
            scope_id: checkpoint_id.to_string(),
            model: config.model.clone(),
            prompt_version: Some(PROMPT_VERSION.to_string()),
            input_digest: Some(hash_input(&input_summary, &config.model)),
            input_summary: Some(input_summary),
            output_json,
            created_at,
        };
        self.checkpoint_repository.insert_llm_run(&run).await?;

        Ok(stored)
    }
}
