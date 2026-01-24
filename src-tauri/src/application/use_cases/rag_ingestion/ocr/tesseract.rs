use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::application::use_cases::rag_config::OcrConfig;

use super::RagIngestionUseCase;

impl RagIngestionUseCase {
    pub(super) fn new_tesseract_command() -> Command {
        let tesseract_cmd =
            std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
        let mut command = Command::new(&tesseract_cmd);
        if let Ok(tessdata_prefix) = std::env::var("TESSDATA_PREFIX") {
            command.env("TESSDATA_PREFIX", tessdata_prefix);
        }
        command
    }

    pub(super) fn run_tesseract_on_image(
        &self,
        image_path: &Path,
        languages: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        let output = Self::new_tesseract_command()
            .arg(image_path.as_os_str())
            .arg("stdout")
            .arg("-l")
            .arg(languages)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    add_log(
                        logs,
                        "WARN",
                        "RAG",
                        &format!("Tesseract failed: {}", stderr.trim()),
                    );
                    None
                }
            }
            Err(err) => {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Tesseract OCR failed to start: {}", err),
                );
                None
            }
        }
    }

    pub(super) fn ocr_image_with_config(
        &self,
        image_path: &Path,
        config: &OcrConfig,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        let languages = if config.languages.trim().is_empty() {
            "eng+ind"
        } else {
            config.languages.as_str()
        };

        let mut preprocessed: Option<PathBuf> = None;
        let mut ocr_path = image_path.to_path_buf();

        if config.preprocessing_enabled {
            let mode = config.preprocessing_mode.as_str();
            if mode == "auto" {
                if self.needs_preprocessing(image_path) {
                    add_log(logs, "INFO", "RAG", "Applying auto OCR preprocessing");
                    if let Some(path) = self.preprocess_image_for_ocr(image_path) {
                        ocr_path = path.clone();
                        preprocessed = Some(path);
                    }
                }
            } else {
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!("Applying OCR preprocessing mode: {}", mode),
                );
                if let Some(path) = self.preprocess_image_for_mode(image_path, mode) {
                    ocr_path = path.clone();
                    preprocessed = Some(path);
                }
            }
        }

        let text = self.run_tesseract_on_image(&ocr_path, languages, logs);
        if let Some(path) = preprocessed {
            let _ = fs::remove_file(path);
        }

        text
    }

    /// OCR a single image file with Tesseract.
    /// Applies preprocessing automatically if contrast looks poor.
    pub(super) fn ocr_single_image(
        &self,
        image_path: &Path,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        let (ocr_path, preprocessed_path) = if self.needs_preprocessing(image_path) {
            add_log(
                logs,
                "INFO",
                "RAG",
                "Applying image preprocessing for better OCR...",
            );
            if let Some(preprocessed) = self.preprocess_image_for_ocr(image_path) {
                (preprocessed.clone(), Some(preprocessed))
            } else {
                (image_path.to_path_buf(), None)
            }
        } else {
            (image_path.to_path_buf(), None)
        };

        let output = Self::new_tesseract_command()
            .arg(ocr_path.as_os_str())
            .arg("stdout")
            .arg("-l")
            .arg("eng+ind")
            .output();

        if let Some(preprocessed) = preprocessed_path {
            let _ = fs::remove_file(preprocessed);
        }

        match output {
            Ok(output) => {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    add_log(
                        logs,
                        "WARN",
                        "RAG",
                        &format!("Tesseract failed: {}", stderr.trim()),
                    );
                    None
                }
            }
            Err(err) => {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Tesseract OCR failed to start: {}", err),
                );
                None
            }
        }
    }
}
