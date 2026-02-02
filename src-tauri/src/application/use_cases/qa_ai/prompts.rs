use crate::application::use_cases::qa_ai::event_text::truncate;
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaTestCase};
use crate::domain::qa_session::QaSession;

pub(crate) fn build_summary_system_prompt(language: &str) -> String {
    format!(
        "You are a QA automation assistant. Summarize the event chunks into a concise checkpoint summary. Respond in {}. Return JSON with keys: summary_text (bullet list), entities (list of fields/buttons/routes), risks (list of anomalies/errors). Return only JSON.",
        language
    )
}

pub(crate) fn build_summary_user_prompt(
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

pub(crate) fn build_test_system_prompt(language: &str) -> String {
    format!(
        "You are a QA automation assistant. Generate negative, edge, and exploratory test cases from the checkpoint summary and event chunks. Respond in {}. Return JSON with arrays: negative_cases, edge_cases, exploratory_charters, api_gap_checks. Each item: {{title, steps, expected, priority}}. Return only JSON.",
        language
    )
}

pub(crate) fn build_test_user_prompt(
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

pub(crate) fn build_explore_system_prompt(language: &str) -> String {
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

pub(crate) fn build_explore_user_prompt(
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
