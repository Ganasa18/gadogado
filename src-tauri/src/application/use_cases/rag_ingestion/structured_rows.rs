use crate::application::use_cases::structured_row_schema::{
    detect_field, is_sensitive_header, normalize_header as normalize_header_schema, StructuredField,
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

#[derive(Debug, Clone)]
pub(super) struct StructuredRowMapping {
    category_idx: Option<usize>,
    source_idx: Option<usize>,
    title_idx: Option<usize>,
    created_at_idx: Option<usize>,
}

#[derive(Debug, Clone)]
pub(super) struct ExtractedStructuredFields {
    pub(super) category: Option<String>,
    pub(super) source: Option<String>,
    pub(super) title: Option<String>,
    pub(super) created_at_text: Option<String>,
    pub(super) created_at: Option<String>,
}

impl StructuredRowMapping {
    pub(super) fn from_header(header: Option<&[String]>) -> Self {
        // Normalizes header fields and picks indices for known columns.
        // We keep this heuristic simple and robust.
        let mut mapping = Self {
            category_idx: None,
            source_idx: None,
            title_idx: None,
            created_at_idx: None,
        };

        let Some(header) = header else {
            return mapping;
        };

        for (idx, name) in header.iter().enumerate() {
            let key = normalize_header_schema(name);

            match detect_field(&key) {
                Some(StructuredField::Category) if mapping.category_idx.is_none() => {
                    mapping.category_idx = Some(idx);
                }
                Some(StructuredField::Source) if mapping.source_idx.is_none() => {
                    mapping.source_idx = Some(idx);
                }
                Some(StructuredField::Title) if mapping.title_idx.is_none() => {
                    mapping.title_idx = Some(idx);
                }
                Some(StructuredField::CreatedAt) if mapping.created_at_idx.is_none() => {
                    mapping.created_at_idx = Some(idx);
                }
                _ => {}
            }
        }

        mapping
    }

    pub(super) fn extract(&self, row: &[String]) -> ExtractedStructuredFields {
        // If we don't have a header mapping (common for header-less CSV exports),
        // fall back to positional mapping for the typical schema:
        // - 6 columns: id, category, source, title, content, created_at
        // - 5 columns: category, source, title, content, created_at
        let fallback = self.category_idx.is_none()
            && self.source_idx.is_none()
            && self.title_idx.is_none()
            && self.created_at_idx.is_none();

        let (category_idx, source_idx, title_idx, created_at_idx) = if fallback {
            if row.len() >= 6 {
                (Some(1), Some(2), Some(3), Some(5))
            } else if row.len() >= 5 {
                (Some(0), Some(1), Some(2), Some(4))
            } else {
                (None, None, None, None)
            }
        } else {
            (
                self.category_idx,
                self.source_idx,
                self.title_idx,
                self.created_at_idx,
            )
        };

        let category = category_idx.and_then(|i| row.get(i)).map(|s| clean_cell(s));
        let source = source_idx.and_then(|i| row.get(i)).map(|s| clean_cell(s));
        let title = title_idx.and_then(|i| row.get(i)).map(|s| clean_cell(s));

        let created_at_text = created_at_idx
            .and_then(|i| row.get(i))
            .map(|s| clean_cell(s));

        let created_at = created_at_text
            .as_deref()
            .and_then(parse_datetime_to_iso)
            .map(|dt| dt);

        ExtractedStructuredFields {
            category: category.filter(|s| !s.is_empty()),
            source: source.filter(|s| !s.is_empty()),
            title: title.filter(|s| !s.is_empty()),
            created_at_text: created_at_text.filter(|s| !s.is_empty()),
            created_at,
        }
    }
}

pub(super) fn split_header_and_rows(
    rows: &[Vec<String>],
) -> (Option<Vec<String>>, Vec<Vec<String>>) {
    if rows.is_empty() {
        return (None, Vec::new());
    }

    let first = &rows[0];
    if looks_like_header(first) {
        (Some(first.clone()), rows[1..].to_vec())
    } else {
        (None, rows.to_vec())
    }
}

fn looks_like_header(row: &[String]) -> bool {
    // Heuristic:
    // - if at least 2 cells match known structured fields (or sensitive tokens)
    // - OR if most cells are non-numeric short tokens

    let mut keyword_hits = 0usize;
    for cell in row {
        let key = normalize_header_schema(cell);
        if detect_field(&key).is_some() || is_sensitive_header(&key) {
            keyword_hits += 1;
        }
    }

    if keyword_hits >= 2 {
        return true;
    }

    let mut non_numeric = 0usize;
    for cell in row {
        let c = cell.trim();
        if c.is_empty() {
            continue;
        }
        if c.parse::<f64>().is_err() {
            non_numeric += 1;
        }
    }

    non_numeric >= 2 && non_numeric >= (row.len().saturating_sub(1))
}

fn clean_cell(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

pub(super) fn build_row_content(header: Option<&[String]>, row: &[String]) -> String {
    // Build a stable, readable string for LLM:
    // - If header exists: "col=value" pairs
    // - Redact sensitive columns (password/token/etc)
    // - Else: join with " | "
    if let Some(header) = header {
        let mut parts = Vec::new();
        for (idx, cell) in row.iter().enumerate() {
            let name_raw = header
                .get(idx)
                .map(|h| clean_cell(h))
                .unwrap_or_else(|| format!("col{}", idx));
            let name_norm = normalize_header_schema(&name_raw);

            let mut val = clean_cell(cell);
            if val.is_empty() {
                continue;
            }

            if is_sensitive_header(&name_norm) {
                val = "[REDACTED]".to_string();
            }

            parts.push(format!("{}={}", name_raw, val.replace('\n', " ")));
        }
        if parts.is_empty() {
            row.join(" | ")
        } else {
            parts.join(" | ")
        }
    } else {
        row.iter()
            .map(|s| clean_cell(s))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

pub(super) fn redact_row_for_storage(header: Option<&[String]>, row: &[String]) -> Vec<String> {
    // Redact sensitive fields in stored JSON payloads.
    // If header is missing we can't reliably detect sensitive columns.
    if let Some(header) = header {
        let mut out = Vec::with_capacity(row.len());
        for (idx, cell) in row.iter().enumerate() {
            let name_raw = header
                .get(idx)
                .map(|h| clean_cell(h))
                .unwrap_or_else(|| format!("col{}", idx));
            let name_norm = normalize_header_schema(&name_raw);

            if is_sensitive_header(&name_norm) {
                out.push("[REDACTED]".to_string());
            } else {
                out.push(clean_cell(cell));
            }
        }
        out
    } else {
        row.iter().map(|s| clean_cell(s)).collect()
    }
}

pub(super) fn parse_datetime_to_iso(raw: &str) -> Option<String> {
    // Try a few common formats. Store as ISO-ish string to keep it consistent.
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    // If already ISO 8601-ish, accept.
    if s.contains('T') && (s.ends_with('Z') || s.contains('+')) {
        return Some(s.to_string());
    }

    // 2026-01-22
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(
            d.and_hms_opt(0, 0, 0)?
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        );
    }

    // 2026/01/22
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        return Some(
            d.and_hms_opt(0, 0, 0)?
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        );
    }

    // 22/01/2026 or 22-01-2026
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d/%m/%Y") {
        return Some(
            d.and_hms_opt(0, 0, 0)?
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        );
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d-%m-%Y") {
        return Some(
            d.and_hms_opt(0, 0, 0)?
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        );
    }

    // 01/22/2026 (US)
    if let Ok(d) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
        return Some(
            d.and_hms_opt(0, 0, 0)?
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
        );
    }

    // 2026-01-22 13:45:00
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // 2026-01-22 13:45
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // 2026/01/22 13:45:00
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M:%S") {
        return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // 2026/01/22 13:45
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M") {
        return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // 22/01/2026 13:45:00
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%d/%m/%Y %H:%M:%S") {
        return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // 22/01/2026 13:45
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%d/%m/%Y %H:%M") {
        return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // Fallback: try parsing as unix timestamp (seconds or milliseconds)
    if let Ok(n) = s.parse::<i64>() {
        if n > 1_000_000_000_000 {
            // ms
            let secs = n / 1000;
            let dt: DateTime<Utc> = DateTime::from_timestamp(secs, 0)?;
            return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        }
        if n > 1_000_000_000 {
            // seconds
            let dt: DateTime<Utc> = DateTime::from_timestamp(n, 0)?;
            return Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        }
    }

    None
}
