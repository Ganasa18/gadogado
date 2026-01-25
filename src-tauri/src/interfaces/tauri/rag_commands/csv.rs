//! CSV Preprocessing Commands
//!
//! This module provides Tauri commands for:
//! - CSV file preprocessing and analysis
//! - Row preview functionality

use crate::application::use_cases::csv_preprocessor::CsvPreprocessor;
use crate::domain::csv::PreprocessingConfig;
use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

use super::types::*;

#[tauri::command]
pub async fn csv_preprocess_file(
    state: State<'_, Arc<super::AppState>>,
    request: CsvPreprocessingRequest,
) -> Result<CsvPreprocessingResponse> {
    add_log(
        &state.logs,
        "INFO",
        "CSV",
        &format!("Starting CSV preprocessing: {}", request.file_path),
    );

    let config = if let Some(req_config) = request.config {
        PreprocessingConfig {
            min_value_length_threshold: req_config
                .min_value_length_threshold
                .unwrap_or(PreprocessingConfig::default().min_value_length_threshold),
            min_lexical_diversity: req_config
                .min_lexical_diversity
                .unwrap_or(PreprocessingConfig::default().min_lexical_diversity),
            max_numeric_ratio: req_config
                .max_numeric_ratio
                .unwrap_or(PreprocessingConfig::default().max_numeric_ratio),
            min_sample_rows: req_config
                .min_sample_rows
                .unwrap_or(PreprocessingConfig::default().min_sample_rows),
            max_sample_rows: req_config
                .max_sample_rows
                .unwrap_or(PreprocessingConfig::default().max_sample_rows),
            ..PreprocessingConfig::default()
        }
    } else {
        PreprocessingConfig::default()
    };

    let preprocessor = CsvPreprocessor::new(config);
    let path = Path::new(&request.file_path);

    let preprocessed = preprocessor.preprocess_csv(path).await.map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "CSV",
            &format!("Preprocessing failed: {}", e),
        );
        e
    })?;

    add_log(
        &state.logs,
        "INFO",
        "CSV",
        &format!(
            "Preprocessing complete: {} rows, {:?} type, {:.2} confidence",
            preprocessed.row_count,
            preprocessed.content_type,
            preprocessed.analysis.confidence_score()
        ),
    );

    Ok(CsvPreprocessingResponse {
        content_type: format!("{:?}", preprocessed.content_type),
        processed_text: preprocessed.processed_text,
        row_count: preprocessed.row_count,
        analysis: CsvFieldAnalysis {
            avg_value_length: preprocessed.analysis.avg_value_length,
            lexical_diversity: preprocessed.analysis.lexical_diversity,
            total_fields: preprocessed.analysis.total_fields,
            numeric_ratio: preprocessed.analysis.numeric_ratio,
            row_count: preprocessed.analysis.row_count,
            empty_field_count: preprocessed.analysis.empty_field_count,
            max_value_length: preprocessed.analysis.max_value_length,
            min_value_length: preprocessed.analysis.min_value_length,
            confidence_score: preprocessed.analysis.confidence_score(),
        },
        headers: preprocessed.headers,
        processing_time_ms: preprocessed.processing_time_ms,
    })
}

/// Preview first N rows of preprocessed CSV

#[tauri::command]
pub async fn csv_preview_rows(
    state: State<'_, Arc<super::AppState>>,
    file_path: String,
    preview_count: usize,
) -> Result<Vec<CsvPreviewRow>> {
    add_log(
        &state.logs,
        "INFO",
        "CSV",
        &format!("Previewing {} rows from: {}", preview_count, file_path),
    );

    let preprocessor = CsvPreprocessor::default();
    let content = std::fs::read_to_string(&file_path).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "CSV",
            &format!("Failed to read file: {}", e),
        );
        crate::domain::error::AppError::IoError(format!("Failed to read file: {}", e))
    })?;

    let preview = preprocessor
        .preview_rows(&content, preview_count)
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "CSV",
                &format!("Preview failed: {}", e),
            );
            e
        })?;

    Ok(preview
        .into_iter()
        .enumerate()
        .map(|(index, content)| CsvPreviewRow { index, content })
        .collect())
}

/// Analyze CSV without full preprocessing

#[tauri::command]
pub async fn csv_analyze(
    state: State<'_, Arc<super::AppState>>,
    file_path: String,
) -> Result<String> {
    add_log(
        &state.logs,
        "INFO",
        "CSV",
        &format!("Analyzing CSV: {}", file_path),
    );

    let preprocessor = CsvPreprocessor::default();
    let content = std::fs::read_to_string(&file_path).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "CSV",
            &format!("Failed to read file: {}", e),
        );
        crate::domain::error::AppError::IoError(format!("Failed to read file: {}", e))
    })?;

    let report = preprocessor.analyze_csv(&content).map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "CSV",
            &format!("Analysis failed: {}", e),
        );
        e
    })?;

    Ok(report)
}

// ============================================================
// DB CONNECTOR COMMANDS
// ============================================================

// Create a new database connection

