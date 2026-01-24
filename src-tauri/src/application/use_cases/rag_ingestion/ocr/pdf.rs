use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::application::use_cases::chunking::PageContent;
use crate::application::use_cases::rag_config::OcrConfig;

use super::RagIngestionUseCase;

struct OcrTempDir {
    path: PathBuf,
}

impl OcrTempDir {
    fn new(prefix: &str) -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!("{}-{}", prefix, uuid::Uuid::new_v4()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for OcrTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl RagIngestionUseCase {
    fn rasterize_pdf_to_pngs(
        &self,
        file_path: &str,
        dpi: u32,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<(OcrTempDir, Vec<PathBuf>)> {
        use crate::interfaces::http::add_log;

        let pdftoppm_cmd = std::env::var("PDFTOPPM_CMD").unwrap_or_else(|_| "pdftoppm".to_string());
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Rasterizing PDF with {}", pdftoppm_cmd),
        );

        let temp_dir = match OcrTempDir::new("gadogado-ocr") {
            Ok(dir) => dir,
            Err(err) => {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to create OCR temp dir: {}", err),
                );
                return None;
            }
        };

        let output_prefix = temp_dir.path.join("page");
        let output = Command::new(&pdftoppm_cmd)
            .arg("-png")
            .arg("-r")
            .arg(dpi.to_string())
            .arg(file_path)
            .arg(output_prefix.to_string_lossy().to_string())
            .output();

        let output = match output {
            Ok(output) => output,
            Err(err) => {
                add_log(
                    logs,
                    "WARN",
                    "RAG",
                    &format!("pdftoppm not available: {}", err),
                );
                return None;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("pdftoppm failed: {}", stderr.trim()),
            );
            return None;
        }

        let mut images: Vec<PathBuf> = match fs::read_dir(&temp_dir.path) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().map(|ext| ext == "png").unwrap_or(false))
                .collect(),
            Err(err) => {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to read OCR temp dir: {}", err),
                );
                return None;
            }
        };

        images.sort();
        if images.is_empty() {
            add_log(logs, "WARN", "RAG", "pdftoppm produced no images");
            return None;
        }

        Some((temp_dir, images))
    }

    pub(super) fn ocr_pdf_with_config(
        &self,
        file_path: &str,
        config: &OcrConfig,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        use crate::interfaces::http::add_log;

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!(
                "Rasterizing PDF with {} for OCR",
                std::env::var("PDFTOPPM_CMD").unwrap_or_else(|_| "pdftoppm".to_string())
            ),
        );

        let (_temp_dir, images) = match self.rasterize_pdf_to_pngs(file_path, 300, logs) {
            Some(v) => v,
            None => {
                let languages = if config.languages.trim().is_empty() {
                    "eng+ind"
                } else {
                    config.languages.as_str()
                };
                return self.ocr_pdf_with_tesseract_fallback_with_lang(file_path, languages, logs);
            }
        };

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Processing {} page images with OCR", images.len()),
        );

        let mut page_contents: Vec<PageContent> = Vec::new();
        for (idx, image_path) in images.iter().enumerate() {
            let page_number = (idx + 1) as i64;
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("OCR processing page {}...", page_number),
            );

            if let Some(text) = self.ocr_image_with_config(image_path, config, logs) {
                if !text.trim().is_empty() {
                    page_contents.push(PageContent {
                        page_number,
                        content: text.trim().to_string(),
                    });
                }
            }
        }

        if page_contents.is_empty() {
            add_log(logs, "WARN", "RAG", "OCR produced no text from any page");
            None
        } else {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("OCR extracted text from {} pages", page_contents.len()),
            );
            Some(page_contents)
        }
    }

    /// OCR PDF pages (legacy method kept for compatibility with the PDF parser).
    pub(in crate::application::use_cases::rag_ingestion) fn ocr_pdf_with_grayscale(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        use crate::interfaces::http::add_log;

        let (_temp_dir, images) = match self.rasterize_pdf_to_pngs(file_path, 300, logs) {
            Some(v) => v,
            None => return self.ocr_pdf_with_tesseract_fallback(file_path, logs),
        };
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Processing {} page images with OCR", images.len()),
        );

        let mut page_contents: Vec<PageContent> = Vec::new();
        for (idx, image_path) in images.iter().enumerate() {
            let page_number = (idx + 1) as i64;
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("OCR processing page {}...", page_number),
            );

            if let Some(text) = self.ocr_single_image(image_path, logs) {
                if !text.trim().is_empty() {
                    page_contents.push(PageContent {
                        page_number,
                        content: text.trim().to_string(),
                    });
                }
            }
        }

        if page_contents.is_empty() {
            add_log(logs, "WARN", "RAG", "OCR produced no text from any page");
            None
        } else {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("OCR extracted text from {} pages", page_contents.len()),
            );
            Some(page_contents)
        }
    }

    fn ocr_pdf_with_tesseract_fallback(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        self.ocr_pdf_with_tesseract_fallback_with_lang(file_path, "eng+ind", logs)
    }

    pub(super) fn ocr_pdf_with_tesseract_fallback_with_lang(
        &self,
        file_path: &str,
        languages: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        use crate::interfaces::http::add_log;

        add_log(logs, "INFO", "RAG", "Trying direct Tesseract OCR on PDF...");
        let output = Self::new_tesseract_command()
            .arg(file_path)
            .arg("stdout")
            .arg("-l")
            .arg(languages)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !text.trim().is_empty() {
                        add_log(logs, "INFO", "RAG", "Direct Tesseract OCR succeeded");
                        return Some(vec![PageContent {
                            page_number: 1,
                            content: text.trim().to_string(),
                        }]);
                    }
                }
                add_log(logs, "WARN", "RAG", "Direct Tesseract OCR produced no text");
                None
            }
            Err(err) => {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Tesseract OCR failed: {}", err),
                );
                None
            }
        }
    }

    pub(super) fn ocr_pdf_with_pdftoppm(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        let (_temp_dir, images) = self.rasterize_pdf_to_pngs(file_path, 300, logs)?;

        let mut combined = String::new();
        for image_path in images {
            match self.run_tesseract_on_image(&image_path, "eng+ind", logs) {
                Some(text) => {
                    if !text.trim().is_empty() {
                        combined.push_str(text.trim());
                        combined.push('\n');
                    }
                }
                None => continue,
            }
        }

        if combined.trim().is_empty() {
            add_log(logs, "WARN", "RAG", "OCR produced empty text");
            None
        } else {
            add_log(logs, "INFO", "RAG", "OCR text extracted successfully");
            Some(combined.trim().to_string())
        }
    }

    pub(super) fn ocr_pdf_with_tesseract(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        add_log(logs, "INFO", "RAG", "Running OCR with Tesseract...");

        if let Some(text) = self.ocr_pdf_with_pdftoppm(file_path, logs) {
            return Some(text);
        }

        let output = Self::new_tesseract_command()
            .arg(file_path)
            .arg("stdout")
            .arg("-l")
            .arg("eng+ind")
            .output();

        let output = match output {
            Ok(output) => output,
            Err(err) => {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Tesseract OCR failed to start: {}", err),
                );
                return None;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Tesseract OCR failed: {}", stderr.trim()),
            );
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        if text.trim().is_empty() {
            add_log(logs, "WARN", "RAG", "OCR produced empty text");
            None
        } else {
            add_log(logs, "INFO", "RAG", "OCR text extracted successfully");
            Some(text.trim().to_string())
        }
    }
}
