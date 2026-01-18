use once_cell::sync::Lazy;
use regex::Regex;

static THINK_TAG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<think>[\s\S]*?</think>|<think\s*/>").unwrap());

static RESULT_PLACEHOLDER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{result\}\}").unwrap());

static REASONING_TAG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<reasoning>[\s\S]*?</reasoning>").unwrap());

static INTERNAL_TAG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<internal>[\s\S]*?</internal>").unwrap());

static MULTIPLE_NEWLINES_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());

/// Cleans LLM response by removing common artifacts and unwanted tags
pub fn clean_llm_response(response: &str) -> String {
    let mut cleaned = response.to_string();

    // Remove <think>...</think> and <think/> tags
    cleaned = THINK_TAG_PATTERN.replace_all(&cleaned, "").to_string();

    // Remove {{result}} placeholders
    cleaned = RESULT_PLACEHOLDER_PATTERN
        .replace_all(&cleaned, "")
        .to_string();

    // Remove <reasoning>...</reasoning> tags (some models use this)
    cleaned = REASONING_TAG_PATTERN.replace_all(&cleaned, "").to_string();

    // Remove <internal>...</internal> tags
    cleaned = INTERNAL_TAG_PATTERN.replace_all(&cleaned, "").to_string();

    // Trim leading/trailing whitespace
    cleaned = cleaned.trim().to_string();

    // Collapse multiple consecutive newlines into at most two
    cleaned = MULTIPLE_NEWLINES_PATTERN
        .replace_all(&cleaned, "\n\n")
        .to_string();

    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_think_tags() {
        let input = "<think>Some reasoning here</think>The actual response";
        assert_eq!(clean_llm_response(input), "The actual response");
    }

    #[test]
    fn test_clean_self_closing_think() {
        let input = "<think/>The actual response";
        assert_eq!(clean_llm_response(input), "The actual response");
    }

    #[test]
    fn test_clean_think_with_space() {
        let input = "<think />The actual response";
        assert_eq!(clean_llm_response(input), "The actual response");
    }

    #[test]
    fn test_clean_result_placeholder() {
        let input = "Here is the result: {{result}}";
        assert_eq!(clean_llm_response(input), "Here is the result:");
    }

    #[test]
    fn test_clean_reasoning_tags() {
        let input = "<reasoning>Internal reasoning</reasoning>Final answer";
        assert_eq!(clean_llm_response(input), "Final answer");
    }

    #[test]
    fn test_clean_internal_tags() {
        let input = "<internal>Debug info</internal>Output";
        assert_eq!(clean_llm_response(input), "Output");
    }

    #[test]
    fn test_clean_multiple_newlines() {
        let input = "Line 1\n\n\n\n\nLine 2";
        assert_eq!(clean_llm_response(input), "Line 1\n\nLine 2");
    }

    #[test]
    fn test_clean_combined() {
        let input = "<think>Let me think...</think>\n\n\n\nHere is the translation: {{result}}\n\nThe actual result";
        assert_eq!(
            clean_llm_response(input),
            "Here is the translation:\n\nThe actual result"
        );
    }

    #[test]
    fn test_clean_preserves_normal_text() {
        let input = "This is a normal response without any special tags.";
        assert_eq!(
            clean_llm_response(input),
            "This is a normal response without any special tags."
        );
    }
}
