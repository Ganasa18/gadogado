use std::collections::HashSet;

use super::language::{avoid_keyword, TargetLanguage};

pub(super) fn sanitize_type_name(input: &str, language: TargetLanguage) -> String {
    let base = to_pascal_case(input);
    let base = if base.is_empty() {
        "Root".to_string()
    } else {
        base
    };
    let base = if starts_with_digit(&base) {
        format!("Type{}", base)
    } else {
        base
    };
    avoid_keyword(base, language)
}

pub(super) fn sanitize_identifier(input: &str, language: TargetLanguage) -> String {
    let base = match language {
        TargetLanguage::Go => to_pascal_case(input),
        TargetLanguage::Rust => to_snake_case(input),
        TargetLanguage::TypeScript => to_lower_camel(input),
        TargetLanguage::Dart => to_lower_camel(input),
        TargetLanguage::Java => to_lower_camel(input),
        TargetLanguage::Php => to_lower_camel(input),
    };

    let base = if base.is_empty() {
        "field".to_string()
    } else {
        base
    };
    let base = if starts_with_digit(&base) {
        format!("field_{}", base)
    } else {
        base
    };
    avoid_keyword(base, language)
}

pub(super) fn unique_name(base: String, used: &mut HashSet<String>) -> String {
    if !used.contains(&base) {
        used.insert(base.clone());
        return base;
    }
    let mut idx = 2;
    loop {
        let candidate = format!("{}{}", base, idx);
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        idx += 1;
    }
}

pub(super) fn ts_property_name(input: &str) -> (String, bool) {
    if is_valid_ts_identifier(input) {
        (input.to_string(), false)
    } else {
        (input.to_string(), true)
    }
}

pub(super) fn escape_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn to_pascal_case(input: &str) -> String {
    let words = split_words(input);
    let mut out = String::new();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for ch in chars {
                out.push(ch.to_ascii_lowercase());
            }
        }
    }
    out
}

fn is_valid_ts_identifier(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn to_lower_camel(input: &str) -> String {
    let pascal = to_pascal_case(input);
    let mut chars = pascal.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_lowercase());
    out.extend(chars);
    out
}

fn to_snake_case(input: &str) -> String {
    let words = split_words(input);
    words
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<String>>()
        .join("_")
}

fn split_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            words.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn starts_with_digit(input: &str) -> bool {
    input
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
}
