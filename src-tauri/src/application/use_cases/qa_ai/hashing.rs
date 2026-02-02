use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) fn hash_input(summary: &str, model: &str) -> String {
    let combined = format!("{}::{}", model, summary);
    hash_value(&combined)
}

pub(crate) fn hash_value(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub(crate) fn normalize_language(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "English".to_string()
    } else {
        trimmed.to_string()
    }
}
