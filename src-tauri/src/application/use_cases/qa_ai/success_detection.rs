use crate::domain::qa_event::QaEvent;
use std::collections::HashSet;

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

/// Detect if a submit event was followed by UI elements indicating success.
/// Returns (has_submit, detected_patterns) where patterns are matched text.
pub(crate) fn detect_post_submit_success(events: &[QaEvent]) -> (bool, Vec<String>) {
    let mut has_submit = false;
    let mut detected_patterns = Vec::new();
    let mut seen_patterns = HashSet::new();

    for event in events {
        if event.event_type == "submit" {
            has_submit = true;
        }

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
