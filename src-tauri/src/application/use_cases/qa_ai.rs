use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaLlmRun, QaTestCase};
use crate::domain::qa_event::{QaEvent, QaEventSummary};
use crate::domain::qa_session::QaSession;
use crate::infrastructure::db::qa_checkpoints::QaCheckpointRepository;
use crate::infrastructure::db::qa_events::QaEventRepository;
use crate::infrastructure::db::qa_sessions::QaRepository;
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::response::clean_llm_response;
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use uuid::Uuid;

const IDLE_THRESHOLD_MS: i64 = 15000;
const MAX_EVENTS_PER_CHUNK: usize = 40;
const PROMPT_VERSION: &str = "v1";

pub struct QaAiUseCase {
    session_repository: Arc<QaRepository>,
    event_repository: Arc<QaEventRepository>,
    checkpoint_repository: Arc<QaCheckpointRepository>,
    llm_client: Arc<dyn LLMClient + Send + Sync>,
}

impl QaAiUseCase {
    pub fn new(
        session_repository: Arc<QaRepository>,
        event_repository: Arc<QaEventRepository>,
        checkpoint_repository: Arc<QaCheckpointRepository>,
        llm_client: Arc<dyn LLMClient + Send + Sync>,
    ) -> Self {
        Self {
            session_repository,
            event_repository,
            checkpoint_repository,
            llm_client,
        }
    }

    pub async fn create_checkpoint(
        &self,
        session_id: &str,
        title: Option<String>,
    ) -> Result<QaCheckpoint> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }

        let latest_event = self
            .event_repository
            .latest_event_summary(session_id)
            .await?
            .ok_or_else(|| AppError::ValidationError("No events recorded yet.".to_string()))?;

        let latest_checkpoint = self
            .checkpoint_repository
            .latest_checkpoint(session_id)
            .await?;
        let start_seq = latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.end_event_seq + 1)
            .unwrap_or(1);

        if start_seq > latest_event.seq {
            return Err(AppError::ValidationError(
                "No new events to create a checkpoint.".to_string(),
            ));
        }

        self.insert_checkpoint(session_id, title, start_seq, latest_event.seq)
            .await
    }

    pub async fn maybe_create_checkpoint_from_event(
        &self,
        session_id: &str,
        recorded: &QaEvent,
        previous_event: Option<QaEventSummary>,
    ) -> Result<Vec<QaCheckpoint>> {
        let mut created = Vec::new();
        let mut latest_checkpoint = self
            .checkpoint_repository
            .latest_checkpoint(session_id)
            .await?;
        let mut start_seq = latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.end_event_seq + 1)
            .unwrap_or(1);

        if let Some(previous) = previous_event {
            if recorded.ts - previous.ts >= IDLE_THRESHOLD_MS && previous.seq >= start_seq {
                let checkpoint = self
                    .insert_checkpoint(
                        session_id,
                        Some("Idle gap checkpoint".to_string()),
                        start_seq,
                        previous.seq,
                    )
                    .await?;
                created.push(checkpoint);
                latest_checkpoint = self
                    .checkpoint_repository
                    .latest_checkpoint(session_id)
                    .await?;
                start_seq = latest_checkpoint
                    .as_ref()
                    .map(|checkpoint| checkpoint.end_event_seq + 1)
                    .unwrap_or(1);
            }
        }

        let event_type = recorded.event_type.as_str();
        let is_submit = event_type == "submit";
        let is_navigation = event_type == "navigation";
        if (is_submit || is_navigation) && recorded.seq >= start_seq {
            let title = if is_submit {
                "Form submit checkpoint".to_string()
            } else {
                "Navigation checkpoint".to_string()
            };
            let checkpoint = self
                .insert_checkpoint(session_id, Some(title), start_seq, recorded.seq)
                .await?;
            created.push(checkpoint);
        }

        Ok(created)
    }

    pub async fn list_checkpoints(&self, session_id: &str) -> Result<Vec<QaCheckpoint>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.checkpoint_repository
            .list_checkpoints(session_id)
            .await
    }

    pub async fn list_checkpoint_summaries(
        &self,
        session_id: &str,
    ) -> Result<Vec<QaCheckpointSummary>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.checkpoint_repository
            .list_checkpoint_summaries(session_id)
            .await
    }

    pub async fn list_test_cases(&self, session_id: &str) -> Result<Vec<QaTestCase>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.checkpoint_repository.list_test_cases(session_id).await
    }

    pub async fn list_llm_runs(&self, session_id: &str) -> Result<Vec<QaLlmRun>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.checkpoint_repository.list_llm_runs(session_id).await
    }

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
            return Err(AppError::ValidationError(
                "Checkpoint has no events.".to_string(),
            ));
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
            return Err(AppError::ValidationError(
                "Checkpoint has no events.".to_string(),
            ));
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

    async fn insert_checkpoint(
        &self,
        session_id: &str,
        title: Option<String>,
        start_event_seq: i64,
        end_event_seq: i64,
    ) -> Result<QaCheckpoint> {
        if start_event_seq <= 0 || end_event_seq <= 0 {
            return Err(AppError::ValidationError(
                "Checkpoint event range is invalid.".to_string(),
            ));
        }
        if start_event_seq > end_event_seq {
            return Err(AppError::ValidationError(
                "Checkpoint start sequence must be before end.".to_string(),
            ));
        }
        let created_at = chrono::Utc::now().timestamp_millis();
        let id = Uuid::new_v4().to_string();
        self.checkpoint_repository
            .insert_checkpoint(
                session_id,
                title
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                start_event_seq,
                end_event_seq,
                created_at,
                id,
            )
            .await
    }

    async fn store_test_cases(
        &self,
        checkpoint_id: &str,
        session: &QaSession,
        items: &[TestCaseInput],
        case_type: &str,
        created_at: i64,
    ) -> Result<Vec<QaTestCase>> {
        let mut stored = Vec::new();
        for item in items {
            let title = item.title.trim();
            if title.is_empty() {
                continue;
            }
            let steps_json =
                serde_json::to_string(&item.steps).unwrap_or_else(|_| "[]".to_string());
            let dedup_source = format!("{}:{}:{}", case_type, title, steps_json);
            let test_case = QaTestCase {
                id: Uuid::new_v4().to_string(),
                session_id: session.id.clone(),
                checkpoint_id: Some(checkpoint_id.to_string()),
                case_type: case_type.to_string(),
                title: title.to_string(),
                steps_json,
                expected: item
                    .expected
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                priority: item
                    .priority
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                status: None,
                dedup_hash: hash_value(&dedup_source),
                created_at,
            };
            self.checkpoint_repository
                .insert_test_case(&test_case)
                .await?;
            stored.push(test_case);
        }
        Ok(stored)
    }

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

        // Detect post-submit success patterns in events
        let (has_submit, detected_patterns) = detect_post_submit_success(&events);

        // Create checkpoint covering all events if none exists
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
                    format!("Login flow with success: {}", detected_patterns.join(", "))
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

        // Generate summary for checkpoint
        let mut generated_summaries = Vec::new();
        let mut generated_llm_runs = Vec::new();

        let chunked = build_chunked_event_text(&events);
        let input_summary = build_input_summary(&session, &checkpoint, chunked.len());
        let language = normalize_language(output_language);

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
        generated_summaries.push(summary);

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
        generated_llm_runs.push(summary_run);

        // Generate test cases using exploration prompt
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

        // Store all generated test cases
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

#[derive(Debug, Deserialize, serde::Serialize)]
struct SummaryOutput {
    summary_text: String,
    entities: Option<Vec<String>>,
    risks: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct TestCaseOutput {
    #[serde(default)]
    negative_cases: Vec<TestCaseInput>,
    #[serde(default)]
    edge_cases: Vec<TestCaseInput>,
    #[serde(default)]
    exploratory_charters: Vec<TestCaseInput>,
    #[serde(default)]
    api_gap_checks: Vec<TestCaseInput>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct TestCaseInput {
    title: String,
    steps: Vec<String>,
    expected: Option<String>,
    priority: Option<String>,
}

/// LLM output for exploration: includes positive case + negatives
#[derive(Debug, Deserialize, serde::Serialize)]
struct ExploreOutput {
    #[serde(default)]
    positive_case: Option<TestCaseInput>,
    #[serde(default)]
    negative_cases: Vec<TestCaseInput>,
    #[serde(default)]
    edge_cases: Vec<TestCaseInput>,
    #[serde(default)]
    exploratory_charters: Vec<TestCaseInput>,
}

/// Result of AI exploration containing all generated database records
#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResult {
    pub checkpoints: Vec<QaCheckpoint>,
    pub summaries: Vec<QaCheckpointSummary>,
    pub test_cases: Vec<QaTestCase>,
    pub llm_runs: Vec<QaLlmRun>,
    pub post_submit_detected: bool,
    pub detected_patterns: Vec<String>,
}

/// Success UI patterns to detect after form submit (case-insensitive)
const SUCCESS_PATTERNS: &[&str] = &[
    "congratulations",
    "successfully",
    "success",
    "welcome",
    "logged in",
    "log out",
    "logout",
    "sign out",
    "signout",
    "dashboard",
    "profile",
    "account",
    "authenticated",
    "thank you",
    "thanks",
];

fn build_chunked_event_text(events: &[QaEvent]) -> Vec<Vec<String>> {
    let mut chunks: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for event in events {
        current.push(format_event_line(event));
        let event_type = event.event_type.as_str();
        let is_boundary = matches!(event_type, "submit" | "navigation")
            || event_type.starts_with("curl_")
            || event_type.starts_with("api_");
        if current.len() >= MAX_EVENTS_PER_CHUNK || is_boundary {
            chunks.push(current);
            current = Vec::new();
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn format_event_line(event: &QaEvent) -> String {
    let mut parts = Vec::new();
    parts.push(format!("#{} {}", event.seq, event.event_type));
    if let Some(selector) = event.selector.as_ref() {
        parts.push(format!("selector={}", truncate(selector, 120)));
    }
    if let Some(text) = event.element_text.as_ref() {
        parts.push(format!("text={}", truncate(text, 120)));
    }
    if let Some(value) = event.value.as_ref() {
        parts.push(format!("value={}", truncate(value, 120)));
    }
    if let Some(url) = event.url.as_ref() {
        parts.push(format!("url={}", truncate(url, 140)));
    }
    if let Some(meta_json) = event.meta_json.as_ref() {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(meta_json) {
            if let Some(method) = meta.get("method").and_then(|value| value.as_str()) {
                parts.push(format!("method={}", method));
            }
            if let Some(status) = meta.get("status") {
                parts.push(format!("status={}", status));
            }
        }
    }
    parts.join(" | ")
}

fn truncate(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= limit {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..limit])
    }
}

fn preview_text(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let snippet: String = trimmed.chars().take(limit).collect();
    if trimmed.chars().count() > limit {
        format!("{}…", snippet)
    } else {
        snippet
    }
}

fn build_input_summary(
    session: &QaSession,
    checkpoint: &QaCheckpoint,
    chunk_count: usize,
) -> String {
    format!(
        "session_id={} session_type={} goal={} checkpoint_seq={} events={} chunks={}",
        session.id,
        session.session_type,
        truncate(&session.goal, 140),
        checkpoint.seq,
        checkpoint.end_event_seq - checkpoint.start_event_seq + 1,
        chunk_count
    )
}

fn build_summary_system_prompt(language: &str) -> String {
    format!(
        "You are a QA automation assistant. Summarize the event chunks into a concise checkpoint summary. Respond in {}. Return JSON with keys: summary_text (bullet list), entities (list of fields/buttons/routes), risks (list of anomalies/errors). Return only JSON.",
        language
    )
}

fn build_summary_user_prompt(
    session: &QaSession,
    checkpoint: &QaCheckpoint,
    chunks: &[Vec<String>],
    language: &str,
) -> String {
    let mut body = String::new();
    body.push_str(&format!("Session goal: {}\n", session.goal));
    body.push_str(&format!("Session type: {}\n", session.session_type));
    body.push_str(&format!("Response language: {}\n", language));

    if let Some(url) = session.target_url.as_ref() {
        body.push_str(&format!("Target URL: {}\n", url));
    }
    if let Some(api_base_url) = session.api_base_url.as_ref() {
        body.push_str(&format!("API base URL: {}\n", api_base_url));
    }
    body.push_str(&format!(
        "Checkpoint seq: {} (events {} to {})\n",
        checkpoint.seq, checkpoint.start_event_seq, checkpoint.end_event_seq
    ));

    body.push_str("\nEvent chunks:\n");
    for (index, chunk) in chunks.iter().enumerate() {
        body.push_str(&format!("\nChunk {}:\n", index + 1));
        for line in chunk {
            body.push_str("- ");
            body.push_str(line);
            body.push('\n');
        }
    }

    body
}

fn build_test_system_prompt(language: &str) -> String {
    format!(
        "You are a QA automation assistant. Generate negative, edge, and exploratory test cases from the checkpoint summary and event chunks. Respond in {}. Return JSON with arrays: negative_cases, edge_cases, exploratory_charters, api_gap_checks. Each item: {{title, steps, expected, priority}}. Return only JSON.",
        language
    )
}

fn build_test_user_prompt(
    session: &QaSession,
    checkpoint: &QaCheckpoint,
    summary: Option<&QaCheckpointSummary>,
    chunks: &[Vec<String>],
    existing_cases: &[QaTestCase],
    language: &str,
) -> String {
    let mut body = String::new();
    body.push_str(&format!("Session goal: {}\n", session.goal));
    body.push_str(&format!("Session type: {}\n", session.session_type));
    body.push_str(&format!("Response language: {}\n", language));
    if let Some(url) = session.target_url.as_ref() {
        body.push_str(&format!("Target URL: {}\n", url));
    }
    if let Some(api_base_url) = session.api_base_url.as_ref() {
        body.push_str(&format!("API base URL: {}\n", api_base_url));
    }
    body.push_str(&format!(
        "Checkpoint seq: {} (events {} to {})\n",
        checkpoint.seq, checkpoint.start_event_seq, checkpoint.end_event_seq
    ));
    if let Some(summary) = summary {
        body.push_str(&format!("Checkpoint summary: {}\n", summary.summary_text));
    }

    if !existing_cases.is_empty() {
        body.push_str("Existing test cases:\n");
        for case in existing_cases {
            body.push_str(&format!(
                "- [{}] {}\n",
                case.case_type,
                truncate(&case.title, 140)
            ));
        }
    }

    body.push_str("\nEvent chunks:\n");
    for (index, chunk) in chunks.iter().enumerate() {
        body.push_str(&format!("\nChunk {}:\n", index + 1));
        for line in chunk {
            body.push_str("- ");
            body.push_str(line);
            body.push('\n');
        }
    }

    body
}

fn hash_input(summary: &str, model: &str) -> String {
    let combined = format!("{}::{}", model, summary);
    hash_value(&combined)
}

fn hash_value(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn normalize_language(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "English".to_string()
    } else {
        trimmed.to_string()
    }
}

fn extract_json_payload(output: &str) -> String {
    let trimmed = output.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(content) = value
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
        {
            return strip_code_fence(content);
        }
        return trimmed.to_string();
    }
    strip_code_fence(trimmed)
}

fn strip_code_fence(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim().to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim().to_string();
    }
    trimmed.to_string()
}

/// Detect if a submit event was followed by UI elements indicating success.
/// Returns (has_submit, detected_patterns) where patterns are matched text.
fn detect_post_submit_success(events: &[QaEvent]) -> (bool, Vec<String>) {
    let mut has_submit = false;
    let mut detected_patterns = Vec::new();
    let mut seen_patterns = std::collections::HashSet::new();

    for event in events {
        if event.event_type == "submit" {
            has_submit = true;
        }

        // Check element_text, value, and selector for success patterns
        let text_sources = [
            event.element_text.as_deref(),
            event.value.as_deref(),
            event.selector.as_deref(),
        ];

        for text in text_sources.iter().filter_map(|t| *t) {
            let lower = text.to_lowercase();
            for pattern in SUCCESS_PATTERNS {
                if lower.contains(pattern) && !seen_patterns.contains(*pattern) {
                    seen_patterns.insert(*pattern);
                    detected_patterns.push(pattern.to_string());
                }
            }
        }

        // Also check meta_json for response data or DOM content
        if let Some(meta_json) = event.meta_json.as_ref() {
            let lower = meta_json.to_lowercase();
            for pattern in SUCCESS_PATTERNS {
                if lower.contains(pattern) && !seen_patterns.contains(*pattern) {
                    seen_patterns.insert(*pattern);
                    detected_patterns.push(pattern.to_string());
                }
            }
        }
    }

    (has_submit, detected_patterns)
}

fn build_explore_system_prompt(language: &str) -> String {
    format!(
        r#"You are a QA automation assistant analyzing a recorded browser flow. The user has recorded a complete positive test case (e.g., a login flow with valid credentials that succeeded).

Your task:
1. Document the positive test case that was recorded (the happy path)
2. Generate negative test cases that should fail (e.g., missing username, missing password, invalid credentials)
3. Generate edge cases to test boundaries
4. Generate exploratory charters for further testing

Respond in {}. Return JSON with:
- positive_case: {{title, steps, expected, priority}} - the recorded happy path
- negative_cases: array of {{title, steps, expected, priority}} - cases that should fail
- edge_cases: array of {{title, steps, expected, priority}} - boundary tests
- exploratory_charters: array of {{title, steps, expected, priority}} - areas to explore

For negative_cases, generate at least:
- Test with empty username
- Test with empty password
- Test with invalid/wrong credentials
- Test with special characters in fields

Return only valid JSON."#,
        language
    )
}

fn build_explore_user_prompt(
    session: &QaSession,
    checkpoint: &QaCheckpoint,
    detected_patterns: &[String],
    chunks: &[Vec<String>],
    language: &str,
) -> String {
    let mut body = String::new();
    body.push_str(&format!("Session goal: {}\n", session.goal));
    body.push_str(&format!("Session type: {}\n", session.session_type));
    body.push_str(&format!("Response language: {}\n", language));

    if let Some(url) = session.target_url.as_ref() {
        body.push_str(&format!("Target URL: {}\n", url));
    }

    body.push_str(&format!(
        "Checkpoint: seq={} (events {} to {})\n",
        checkpoint.seq, checkpoint.start_event_seq, checkpoint.end_event_seq
    ));

    if !detected_patterns.is_empty() {
        body.push_str(&format!(
            "\nPost-submit success indicators detected: {}\n",
            detected_patterns.join(", ")
        ));
        body.push_str("This indicates the recorded flow was a successful positive case.\n");
    }

    body.push_str("\nRecorded events (this is the positive/happy path):\n");
    for (index, chunk) in chunks.iter().enumerate() {
        body.push_str(&format!("\nChunk {}:\n", index + 1));
        for line in chunk {
            body.push_str("- ");
            body.push_str(line);
            body.push('\n');
        }
    }

    body.push_str("\nBased on this recorded positive flow, generate:\n");
    body.push_str("1. A positive_case documenting what was recorded\n");
    body.push_str("2. negative_cases that should FAIL (empty fields, wrong credentials, etc.)\n");
    body.push_str("3. edge_cases for boundary testing\n");
    body.push_str("4. exploratory_charters for additional testing areas\n");

    body
}
