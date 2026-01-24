use super::{ExploreResult, QaAiUseCase, PROMPT_VERSION};
use crate::application::use_cases::qa_ai::event_text::{
    build_chunked_event_text, build_input_summary, preview_text,
};
use crate::application::use_cases::qa_ai::hashing::{hash_input, normalize_language};
use crate::application::use_cases::qa_ai::llm_output::extract_json_payload;
use crate::application::use_cases::qa_ai::prompts::{
    build_explore_system_prompt, build_explore_user_prompt, build_summary_system_prompt,
    build_summary_user_prompt,
};
use crate::application::use_cases::qa_ai::success_detection::detect_post_submit_success;
use crate::application::use_cases::qa_ai::types::{ExploreOutput, SummaryOutput};
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::qa_checkpoint::QaLlmRun;
use crate::infrastructure::response::clean_llm_response;
use uuid::Uuid;

impl QaAiUseCase {
    /// Explore recorded events and generate test cases for a complete positive flow.
    /// Detects post-submit UI confirmation patterns (without requiring URL changes).
    pub async fn explore_and_generate_tests(
        &self,
        session_id: &str,
        config: &LLMConfig,
        output_language: &str,
    ) -> Result<ExploreResult> {
        let session = self.session_repository.get_session(session_id).await?;
        let events = self.event_repository.list_events(session_id).await?;
        if events.is_empty() {
            return Err(AppError::ValidationError(
                "No events recorded in this session.".to_string(),
            ));
        }

        let (has_submit, detected_patterns) = detect_post_submit_success(&events);

        let mut generated_checkpoints = Vec::new();
        let checkpoint = match self
            .checkpoint_repository
            .latest_checkpoint(session_id)
            .await?
        {
            Some(existing)
                if existing.end_event_seq >= events.last().map(|e| e.seq).unwrap_or(0) =>
            {
                existing
            }
            _ => {
                let first_seq = events.first().map(|e| e.seq).unwrap_or(1);
                let last_seq = events.last().map(|e| e.seq).unwrap_or(1);
                let title = if has_submit && !detected_patterns.is_empty() {
                    format!(
                        "Login flow with success: {}",
                        detected_patterns.join(", ")
                    )
                } else if has_submit {
                    "Login flow (submit detected)".to_string()
                } else {
                    "Recorded browser flow".to_string()
                };
                let cp = self
                    .insert_checkpoint(session_id, Some(title), first_seq, last_seq)
                    .await?;
                generated_checkpoints.push(cp.clone());
                cp
            }
        };

        let chunked = build_chunked_event_text(&events);
        let input_summary = build_input_summary(&session, &checkpoint, chunked.len());
        let language = normalize_language(output_language);

        // Summary generation
        let summary_system = build_summary_system_prompt(&language);
        let summary_user = build_summary_user_prompt(&session, &checkpoint, &chunked, &language);
        let summary_raw = self
            .llm_client
            .generate(config, &summary_system, &summary_user)
            .await?;
        let summary_cleaned = clean_llm_response(&summary_raw);
        let summary_normalized = extract_json_payload(&summary_cleaned);

        let summary_parsed = serde_json::from_str::<SummaryOutput>(&summary_normalized).ok();
        let summary_text = summary_parsed
            .as_ref()
            .map(|o| o.summary_text.clone())
            .unwrap_or_else(|| summary_cleaned.clone());
        let entities_json = summary_parsed
            .as_ref()
            .and_then(|o| o.entities.as_ref())
            .and_then(|items| serde_json::to_string(items).ok());
        let risks_json = summary_parsed
            .as_ref()
            .and_then(|o| o.risks.as_ref())
            .and_then(|items| serde_json::to_string(items).ok());

        let created_at = chrono::Utc::now().timestamp_millis();
        let summary_id = Uuid::new_v4().to_string();
        let summary = self
            .checkpoint_repository
            .insert_checkpoint_summary(
                summary_id,
                &checkpoint.id,
                summary_text,
                entities_json,
                risks_json,
                created_at,
            )
            .await?;
        let generated_summaries = vec![summary];

        let summary_output_json = summary_parsed
            .as_ref()
            .and_then(|o| serde_json::to_string(o).ok())
            .unwrap_or_else(|| summary_normalized.clone());
        let summary_run = QaLlmRun {
            id: Uuid::new_v4().to_string(),
            scope: "checkpoint_summary".to_string(),
            scope_id: checkpoint.id.clone(),
            model: config.model.clone(),
            prompt_version: Some(PROMPT_VERSION.to_string()),
            input_digest: Some(hash_input(&input_summary, &config.model)),
            input_summary: Some(input_summary.clone()),
            output_json: summary_output_json,
            created_at,
        };
        self.checkpoint_repository
            .insert_llm_run(&summary_run)
            .await?;
        let mut generated_llm_runs = vec![summary_run];

        // Exploration test generation
        let explore_system = build_explore_system_prompt(&language);
        let explore_user = build_explore_user_prompt(
            &session,
            &checkpoint,
            &detected_patterns,
            &chunked,
            &language,
        );
        let explore_raw = self
            .llm_client
            .generate(config, &explore_system, &explore_user)
            .await?;
        let explore_cleaned = clean_llm_response(&explore_raw);
        let explore_normalized = extract_json_payload(&explore_cleaned);
        let explore_parsed =
            serde_json::from_str::<ExploreOutput>(&explore_normalized).map_err(|err| {
                let snippet = preview_text(&explore_normalized, 600);
                AppError::Internal(format!(
                    "Failed to parse LLM explore output: {} | output_snippet={}",
                    err, snippet
                ))
            })?;

        let explore_output_json =
            serde_json::to_string(&explore_parsed).unwrap_or_else(|_| explore_normalized.clone());
        let explore_run = QaLlmRun {
            id: Uuid::new_v4().to_string(),
            scope: "explore_tests".to_string(),
            scope_id: checkpoint.id.clone(),
            model: config.model.clone(),
            prompt_version: Some(PROMPT_VERSION.to_string()),
            input_digest: Some(hash_input(&input_summary, &config.model)),
            input_summary: Some(input_summary),
            output_json: explore_output_json,
            created_at,
        };
        self.checkpoint_repository
            .insert_llm_run(&explore_run)
            .await?;
        generated_llm_runs.push(explore_run);

        let mut generated_test_cases = Vec::new();
        if let Some(positive) = explore_parsed.positive_case {
            generated_test_cases.extend(
                self.store_test_cases(
                    &checkpoint.id,
                    &session,
                    &[positive],
                    "positive",
                    created_at,
                )
                .await?,
            );
        }
        generated_test_cases.extend(
            self.store_test_cases(
                &checkpoint.id,
                &session,
                &explore_parsed.negative_cases,
                "negative",
                created_at,
            )
            .await?,
        );
        generated_test_cases.extend(
            self.store_test_cases(
                &checkpoint.id,
                &session,
                &explore_parsed.edge_cases,
                "edge",
                created_at,
            )
            .await?,
        );
        generated_test_cases.extend(
            self.store_test_cases(
                &checkpoint.id,
                &session,
                &explore_parsed.exploratory_charters,
                "exploratory",
                created_at,
            )
            .await?,
        );

        Ok(ExploreResult {
            checkpoints: generated_checkpoints,
            summaries: generated_summaries,
            test_cases: generated_test_cases,
            llm_runs: generated_llm_runs,
            post_submit_detected: has_submit && !detected_patterns.is_empty(),
            detected_patterns,
        })
    }
}
