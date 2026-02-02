// Centralized header alias configuration for structured_rows ingestion.
//
// Goal: keep CSV/XLSX header matching flexible without scattering alias lists.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredField {
    Category,
    Source,
    Title,
    CreatedAt,
}

// NOTE:
// - These aliases are matched against a normalized header (lowercase, space/dash -> underscore).
// - Matching strategy:
//   1) exact match
//   2) ends_with("_alias") or starts_with("alias_")
//   3) contains("_alias_")

pub const CATEGORY_ALIASES: &[&str] = &["category", "kategori", "type", "jenis", "role"];

pub const SOURCE_ALIASES: &[&str] = &[
    "source",
    "sumber",
    "origin",
    "publisher",
    "provider",
    "site",
    "website",
    "domain",
    "url",
    "link",
    "profile_url",
    "daerah",
    "location",
    "lokasi",
    "city",
    "kota",
];

pub const TITLE_ALIASES: &[&str] = &[
    "title",
    "judul",
    "name",
    "nama",
    "full_name",
    "headline",
    "topic",
    "subject",
];

pub const CREATED_AT_ALIASES: &[&str] = &[
    "created_at",
    "created",
    "date",
    "tanggal",
    "time",
    "waktu",
    "timestamp",
    "published",
    "published_at",
    "publish_date",
    // NOTE: we intentionally do NOT include updated_at/deleted_at here.
];

pub const SENSITIVE_HEADER_TOKENS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "token",
    "secret",
    "api_key",
    "apikey",
    "access_key",
    "private_key",
    "credential",
];

pub fn normalize_header(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .to_ascii_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
}

pub fn is_sensitive_header(normalized_header: &str) -> bool {
    // Token match with boundary-ish behavior.
    // Examples matched:
    // - password
    // - login_token
    // - reset_link_token
    for t in SENSITIVE_HEADER_TOKENS {
        if normalized_header == *t {
            return true;
        }
        if normalized_header.ends_with(&format!("_{}", t)) {
            return true;
        }
        if normalized_header.starts_with(&format!("{}_", t)) {
            return true;
        }
        if normalized_header.contains(&format!("_{}_", t)) {
            return true;
        }
    }
    false
}

pub fn header_matches_alias(normalized_header: &str, alias: &str) -> bool {
    if normalized_header == alias {
        return true;
    }
    if normalized_header.ends_with(&format!("_{}", alias)) {
        return true;
    }
    if normalized_header.starts_with(&format!("{}_", alias)) {
        return true;
    }
    if normalized_header.contains(&format!("_{}_", alias)) {
        return true;
    }
    false
}

pub fn detect_field(normalized_header: &str) -> Option<StructuredField> {
    // Priority matters.
    if CREATED_AT_ALIASES
        .iter()
        .any(|a| header_matches_alias(normalized_header, a))
    {
        return Some(StructuredField::CreatedAt);
    }
    if CATEGORY_ALIASES
        .iter()
        .any(|a| header_matches_alias(normalized_header, a))
    {
        return Some(StructuredField::Category);
    }
    if SOURCE_ALIASES
        .iter()
        .any(|a| header_matches_alias(normalized_header, a))
    {
        return Some(StructuredField::Source);
    }
    if TITLE_ALIASES
        .iter()
        .any(|a| header_matches_alias(normalized_header, a))
    {
        return Some(StructuredField::Title);
    }
    None
}
