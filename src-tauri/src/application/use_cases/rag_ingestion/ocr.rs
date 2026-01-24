use std::path::Path;

use crate::application::use_cases::rag_config::OcrConfig;

use super::{AppError, OcrPage, OcrResult, RagIngestionUseCase, Result};

mod pdf;
mod preprocess;
mod tesseract;
mod web;

impl RagIngestionUseCase {
    /// Run OCR on a file without ingestion, using the provided OCR config.
    pub async fn enhanced_ocr(
        &self,
        file_path: &str,
        config: &OcrConfig,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<OcrResult> {
        use crate::interfaces::http::add_log;

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(AppError::Internal(format!(
                "OCR file not found: {}",
                file_path
            )));
        }

        let engine_used = match config.engine.as_str() {
            "tesseract" => "tesseract",
            "auto" => {
                add_log(&logs, "INFO", "RAG", "OCR engine auto-selected: tesseract");
                "tesseract"
            }
            "paddle" => {
                add_log(
                    &logs,
                    "WARN",
                    "RAG",
                    "PaddleOCR not available, falling back to tesseract",
                );
                "tesseract"
            }
            other => {
                add_log(
                    &logs,
                    "WARN",
                    "RAG",
                    &format!("Unknown OCR engine '{}', falling back to tesseract", other),
                );
                "tesseract"
            }
        };

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let result = match ext.as_str() {
            "pdf" => {
                let pages = self
                    .ocr_pdf_with_config(file_path, config, &logs)
                    .unwrap_or_default();
                let total_pages = pages.len();
                let joined = pages
                    .iter()
                    .map(|p| p.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let page_entries = pages
                    .into_iter()
                    .map(|p| OcrPage {
                        page_number: p.page_number,
                        text: p.content,
                    })
                    .collect();
                OcrResult {
                    text: joined,
                    pages: Some(page_entries),
                    total_pages,
                    engine: engine_used.to_string(),
                    preprocessing_mode: config.preprocessing_mode.clone(),
                    preprocessing_enabled: config.preprocessing_enabled,
                }
            }
            "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "tif" => {
                let text = self
                    .ocr_image_with_config(path, config, &logs)
                    .unwrap_or_default();
                OcrResult {
                    text: text.clone(),
                    pages: Some(vec![OcrPage {
                        page_number: 1,
                        text,
                    }]),
                    total_pages: 1,
                    engine: engine_used.to_string(),
                    preprocessing_mode: config.preprocessing_mode.clone(),
                    preprocessing_enabled: config.preprocessing_enabled,
                }
            }
            _ => {
                return Err(AppError::Internal(format!(
                    "Unsupported OCR file type: {}",
                    ext
                )))
            }
        };

        Ok(result)
    }
}
