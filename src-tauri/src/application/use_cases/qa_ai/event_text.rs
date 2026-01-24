use crate::domain::qa_checkpoint::QaCheckpoint;
use crate::domain::qa_event::QaEvent;
use crate::domain::qa_session::QaSession;

pub(crate) const MAX_EVENTS_PER_CHUNK: usize = 40;

pub(crate) fn build_chunked_event_text(events: &[QaEvent]) -> Vec<Vec<String>> {
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

pub(crate) fn build_input_summary(
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

pub(crate) fn preview_text(value: &str, limit: usize) -> String {
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

pub(crate) fn truncate(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= limit {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..limit])
    }
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
