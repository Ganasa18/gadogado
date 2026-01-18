use crate::application::use_cases::chunking::{ChunkEngine, PageContent};
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::rag_config::OcrConfig;
use crate::application::use_cases::web_crawler::{WebCrawler, WebOcrCapture};
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::rag_entities::{
    RagDocument, RagDocumentChunkInput, RagDocumentInput, RagExcelDataInput,
};
use crate::infrastructure::db::rag::repository::RagRepository;
use image::{GrayImage, ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Result type for parsing: (pages_content, total_pages, excel_data)
/// pages_content: Vec of PageContent for documents with page structure (PDF, DOCX)
/// or plain text for single-page documents (TXT, web)
type ParseResult = Result<(ParsedContent, i64, Option<Vec<Vec<String>>>)>;

#[derive(Debug)]
pub enum ParsedContent {
    /// Content with page structure (PDF, DOCX with page breaks)
    Pages(Vec<PageContent>),
    /// Plain text without page structure (TXT, web, single-page docs)
    Plain(Option<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPage {
    pub page_number: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub pages: Option<Vec<OcrPage>>,
    pub total_pages: usize,
    pub engine: String,
    pub preprocessing_mode: String,
    pub preprocessing_enabled: bool,
}

pub struct RagIngestionUseCase {
    rag_repository: Arc<RagRepository>,
    embedding_service: Arc<EmbeddingService>,
    chunk_engine: ChunkEngine,
}

impl RagIngestionUseCase {
    pub fn new(rag_repository: Arc<RagRepository>, config: LLMConfig) -> Self {
        Self {
            rag_repository,
            embedding_service: Arc::new(EmbeddingService::new(config)),
            chunk_engine: ChunkEngine::default(),
        }
    }

    pub fn with_embedding_service(
        rag_repository: Arc<RagRepository>,
        embedding_service: Arc<EmbeddingService>,
    ) -> Self {
        Self {
            rag_repository,
            embedding_service,
            chunk_engine: ChunkEngine::default(),
        }
    }

    // ============================================================
    // IMAGE PREPROCESSING FOR OCR ENHANCEMENT
    // ============================================================

    /// Preprocess an image for better OCR results
    /// Applies: grayscale, contrast enhancement, adaptive thresholding
    fn preprocess_image_for_ocr(&self, image_path: &Path) -> Option<std::path::PathBuf> {
        // Load the image
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(_) => return None,
        };

        // Convert to grayscale
        let gray = img.to_luma8();

        // Apply contrast enhancement
        let enhanced = self.enhance_contrast(&gray);

        // Apply adaptive thresholding (Otsu's method approximation)
        let thresholded = self.adaptive_threshold(&enhanced);

        // Save preprocessed image
        let preprocessed_path = image_path.with_extension("preprocessed.png");
        if thresholded.save(&preprocessed_path).is_ok() {
            Some(preprocessed_path)
        } else {
            None
        }
    }

    /// Enhance contrast using histogram stretching
    fn enhance_contrast(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = ImageBuffer::new(width, height);

        // Find min and max pixel values
        let mut min_val = 255u8;
        let mut max_val = 0u8;

        for pixel in img.pixels() {
            let val = pixel[0];
            if val < min_val {
                min_val = val;
            }
            if val > max_val {
                max_val = val;
            }
        }

        // Avoid division by zero
        let range = if max_val > min_val {
            (max_val - min_val) as f32
        } else {
            1.0
        };

        // Stretch histogram
        for (x, y, pixel) in img.enumerate_pixels() {
            let val = pixel[0];
            let stretched = ((val as f32 - min_val as f32) / range * 255.0) as u8;
            result.put_pixel(x, y, Luma([stretched]));
        }

        result
    }

    /// Apply adaptive thresholding (simplified Otsu's method)
    fn adaptive_threshold(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = ImageBuffer::new(width, height);

        // Calculate histogram
        let mut histogram = [0u32; 256];
        let total_pixels = (width * height) as f64;

        for pixel in img.pixels() {
            histogram[pixel[0] as usize] += 1;
        }

        // Otsu's method to find optimal threshold
        let mut sum = 0.0;
        for (i, &count) in histogram.iter().enumerate() {
            sum += i as f64 * count as f64;
        }

        let mut sum_b = 0.0;
        let mut weight_b = 0.0;
        let mut max_variance = 0.0;
        let mut threshold = 128u8;

        for (i, &count) in histogram.iter().enumerate() {
            weight_b += count as f64;
            if weight_b == 0.0 {
                continue;
            }

            let weight_f = total_pixels - weight_b;
            if weight_f == 0.0 {
                break;
            }

            sum_b += i as f64 * count as f64;
            let mean_b = sum_b / weight_b;
            let mean_f = (sum - sum_b) / weight_f;

            let variance = weight_b * weight_f * (mean_b - mean_f).powi(2);
            if variance > max_variance {
                max_variance = variance;
                threshold = i as u8;
            }
        }

        // Apply threshold
        for (x, y, pixel) in img.enumerate_pixels() {
            let val = if pixel[0] > threshold { 255 } else { 0 };
            result.put_pixel(x, y, Luma([val]));
        }

        result
    }

    /// Estimate if an image needs preprocessing based on contrast analysis
    fn needs_preprocessing(&self, image_path: &Path) -> bool {
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(_) => return false,
        };

        let gray = img.to_luma8();

        // Calculate standard deviation of pixel values
        let pixels: Vec<f64> = gray.pixels().map(|p| p[0] as f64).collect();
        let mean: f64 = pixels.iter().sum::<f64>() / pixels.len() as f64;
        let variance: f64 =
            pixels.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / pixels.len() as f64;
        let std_dev = variance.sqrt();

        // If standard deviation is low, image has poor contrast and needs preprocessing
        std_dev < 50.0
    }

    // ============================================================
    // OCR PREVIEW / ENHANCED OCR (Phase 6)
    // ============================================================

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

    fn preprocess_image_for_mode(&self, image_path: &Path, mode: &str) -> Option<PathBuf> {
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(_) => return None,
        };

        let gray = img.to_luma8();
        let processed = match mode {
            "grayscale" => gray,
            "contrast" => self.enhance_contrast(&gray),
            "otsu" => self.adaptive_threshold(&self.enhance_contrast(&gray)),
            _ => self.adaptive_threshold(&self.enhance_contrast(&gray)),
        };

        let preprocessed_path = image_path.with_extension("preprocessed.png");
        if processed.save(&preprocessed_path).is_ok() {
            Some(preprocessed_path)
        } else {
            None
        }
    }

    fn run_tesseract_on_image(
        &self,
        image_path: &Path,
        languages: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        let tesseract_cmd =
            std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
        let mut command = Command::new(&tesseract_cmd);
        if let Ok(tessdata_prefix) = std::env::var("TESSDATA_PREFIX") {
            command.env("TESSDATA_PREFIX", tessdata_prefix);
        }
        let output = command
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

    fn ocr_image_with_config(
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

    fn ocr_pdf_with_config(
        &self,
        file_path: &str,
        config: &OcrConfig,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        use crate::interfaces::http::add_log;

        let pdftoppm_cmd = std::env::var("PDFTOPPM_CMD").unwrap_or_else(|_| "pdftoppm".to_string());
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Rasterizing PDF with {} for OCR", pdftoppm_cmd),
        );

        let output_dir =
            std::env::temp_dir().join(format!("gadogado-ocr-{}", uuid::Uuid::new_v4()));
        if let Err(err) = fs::create_dir_all(&output_dir) {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to create OCR temp dir: {}", err),
            );
            return None;
        }

        let output_prefix = output_dir.join("page");
        let output = Command::new(&pdftoppm_cmd)
            .arg("-png")
            .arg("-r")
            .arg("300")
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
                let _ = fs::remove_dir_all(&output_dir);
                let languages = if config.languages.trim().is_empty() {
                    "eng+ind"
                } else {
                    config.languages.as_str()
                };
                return self.ocr_pdf_with_tesseract_fallback_with_lang(file_path, languages, logs);
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
            let _ = fs::remove_dir_all(&output_dir);
            return None;
        }

        let mut images: Vec<_> = match fs::read_dir(&output_dir) {
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
                let _ = fs::remove_dir_all(&output_dir);
                return None;
            }
        };

        images.sort();
        if images.is_empty() {
            add_log(logs, "WARN", "RAG", "pdftoppm produced no images");
            let _ = fs::remove_dir_all(&output_dir);
            return None;
        }

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

        let _ = fs::remove_dir_all(&output_dir);

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

    pub async fn ingest_file(
        &self,
        file_path: &str,
        collection_id: Option<i64>,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<RagDocument> {
        use crate::interfaces::http::add_log;

        let path = Path::new(file_path);

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!(
                "Starting import: {}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            ),
        );

        if !path.exists() {
            add_log(
                &logs,
                "ERROR",
                "RAG",
                &format!("File not found: {}", file_path),
            );
            return Err(AppError::NotFound(format!("File not found: {}", file_path)));
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::ValidationError("Invalid file name".to_string()))?
            .to_string();

        let file_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| AppError::ValidationError("Invalid file extension".to_string()))?
            .to_lowercase();

        let file_type = match file_extension.as_str() {
            "pdf" => "pdf",
            "docx" => "docx",
            "xlsx" => "xlsx",
            "csv" => "csv",
            "txt" => "txt",
            "md" => "md",
            "web" => "web",
            _ => {
                add_log(
                    &logs,
                    "ERROR",
                    "RAG",
                    &format!("Unsupported file type: {}", file_extension),
                );
                return Err(AppError::ValidationError(format!(
                    "Unsupported file type: {}",
                    file_extension
                )));
            }
        };

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Parsing {} file...", file_type.to_uppercase()),
        );

        let (parsed_content, pages, excel_data) = match file_type {
            "pdf" => self.parse_pdf(file_path, &logs)?,
            "docx" => self.parse_docx(file_path, &logs)?,
            "xlsx" => self.parse_xlsx(file_path, &logs)?,
            "csv" => self.parse_csv(file_path, &logs)?,
            "txt" | "md" => self.parse_txt(file_path, &logs)?,
            "web" => self.parse_web(file_path, &logs).await?,
            _ => unreachable!(),
        };

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Parsed file: {} pages", pages),
        );

        let document_input = RagDocumentInput {
            collection_id,
            file_name: file_name.clone(),
            file_path: Some(file_path.to_string()),
            file_type: file_type.to_string(),
            language: Some("auto".to_string()),
            total_pages: Some(pages),
        };

        add_log(&logs, "INFO", "RAG", "Creating document record...");

        let document = self.rag_repository.create_document(&document_input).await?;
        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Document created with ID: {}", document.id),
        );

        if let Some(excel_rows) = excel_data {
            add_log(
                &logs,
                "INFO",
                "RAG",
                &format!("Storing {} Excel rows...", excel_rows.len()),
            );
            for (row_index, data) in excel_rows.iter().enumerate() {
                let excel_input = RagExcelDataInput {
                    doc_id: document.id,
                    row_index: row_index as i64,
                    data_json: Some(serde_json::to_string::<Vec<String>>(data).map_err(|e| {
                        AppError::Internal(format!("Failed to serialize Excel data: {}", e))
                    })?),
                    val_a: data.get(0).map(|s| s.to_string()),
                    val_b: data.get(1).map(|s| s.to_string()),
                    val_c: data.get(2).and_then(|s| s.parse::<f64>().ok()),
                };

                self.rag_repository
                    .create_excel_data(&excel_input)
                    .await
                    .map_err(|e| {
                        add_log(
                            &logs,
                            "ERROR",
                            "RAG",
                            &format!("Failed to store Excel row {}: {}", row_index + 1, e),
                        );
                        AppError::Internal(format!("Failed to store Excel row: {}", e))
                    })?;
            }
            add_log(&logs, "INFO", "RAG", "Excel data stored");
        }

        self.store_chunks_for_document(&document, parsed_content, &file_name, file_type, &logs)
            .await?;

        Ok(document)
    }

    pub async fn ingest_web_html(
        &self,
        url: &str,
        collection_id: Option<i64>,
        max_pages: Option<usize>,
        max_depth: Option<usize>,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<RagDocument> {
        use crate::interfaces::http::add_log;

        let max_pages = max_pages.unwrap_or(10);
        let max_depth = max_depth.unwrap_or(2);

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Starting HTML web import: {}", url),
        );

        let file_name = url
            .split('/')
            .last()
            .and_then(|segment| {
                let trimmed = segment.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .unwrap_or("web")
            .split('?')
            .next()
            .unwrap_or("web")
            .to_string();

        let (parsed_content, pages, _) = self
            .parse_web_with_options(url, &logs, max_pages, max_depth)
            .await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Parsed web content: {} pages", pages),
        );

        let document_input = RagDocumentInput {
            collection_id,
            file_name: file_name.clone(),
            file_path: Some(url.to_string()),
            file_type: "web".to_string(),
            language: Some("auto".to_string()),
            total_pages: Some(pages),
        };

        add_log(&logs, "INFO", "RAG", "Creating document record...");

        let document = self.rag_repository.create_document(&document_input).await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Document created with ID: {}", document.id),
        );

        self.store_chunks_for_document(&document, parsed_content, &file_name, "web", &logs)
            .await?;

        Ok(document)
    }

    async fn store_chunks_for_document(
        &self,
        document: &RagDocument,
        parsed_content: ParsedContent,
        file_name: &str,
        file_type: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<()> {
        use crate::interfaces::http::add_log;

        let chunks = match parsed_content {
            ParsedContent::Pages(ref page_contents) => {
                let total_chars: usize = page_contents.iter().map(|p| p.content.len()).sum();
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!(
                        "Chunking {} pages ({} chars total) with page tracking...",
                        page_contents.len(),
                        total_chars
                    ),
                );

                self.chunk_engine
                    .chunk_pages(page_contents)
                    .map_err(|e| AppError::Internal(format!("Failed to chunk pages: {}", e)))?
            }
            ParsedContent::Plain(Some(ref text_content)) => {
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!("Chunking plain text ({} chars)...", text_content.len()),
                );

                self.chunk_engine
                    .chunk_text(text_content)
                    .map_err(|e| AppError::Internal(format!("Failed to chunk text: {}", e)))?
            }
            ParsedContent::Plain(None) => {
                add_log(
                    logs,
                    "WARN",
                    "RAG",
                    &format!(
                        "No text extracted from {} ({}); embeddings skipped",
                        file_name, file_type
                    ),
                );
                add_log(logs, "INFO", "RAG", "Import completed (no text content)");
                return Ok(());
            }
        };

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Created {} chunks", chunks.len()),
        );

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let page_info = chunk
                .page_number
                .map(|p| format!(" (page {})", p))
                .unwrap_or_default();
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Processing chunk {}/{}{}...",
                    chunk_index + 1,
                    chunks.len(),
                    page_info
                ),
            );

            let chunk_input = RagDocumentChunkInput {
                doc_id: document.id,
                content: chunk.content.clone(),
                page_number: chunk.page_number,
                page_offset: chunk.page_offset,
                chunk_index: chunk_index as i64,
                token_count: Some(chunk.token_count as i64),
            };

            let created_chunk = self
                .rag_repository
                .create_chunk(&chunk_input)
                .await
                .map_err(|e| {
                    add_log(
                        logs,
                        "ERROR",
                        "RAG",
                        &format!("Failed to store chunk {}: {}", chunk_index + 1, e),
                    );
                    AppError::Internal(format!("Failed to store chunk: {}", e))
                })?;

            // Update chunk quality if available
            if let Some(quality) = chunk.quality_score {
                let _ = self
                    .rag_repository
                    .update_chunk_quality(
                        created_chunk.id,
                        quality as f64,
                        chunk.content_type.as_deref(),
                    )
                    .await;
            }

            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("Generating embedding for chunk {}...", chunk_index + 1),
            );

            match self
                .embedding_service
                .generate_embedding(&chunk.content)
                .await
            {
                Ok(embedding) => {
                    let embedding_bytes = EmbeddingService::embedding_to_bytes(&embedding);
                    self.rag_repository
                        .update_chunk_embedding(created_chunk.id, &embedding_bytes)
                        .await
                        .map_err(|e| {
                            add_log(
                                logs,
                                "ERROR",
                                "RAG",
                                &format!(
                                    "Failed to update chunk embedding {}: {}",
                                    chunk_index + 1,
                                    e
                                ),
                            );
                            AppError::Internal(format!("Failed to update chunk embedding: {}", e))
                        })?;
                    add_log(
                        logs,
                        "INFO",
                        "RAG",
                        &format!("Chunk {}/{} processed", chunk_index + 1, chunks.len()),
                    );
                }
                Err(e) => {
                    add_log(
                        logs,
                        "ERROR",
                        "RAG",
                        &format!(
                            "Failed to generate embedding for chunk {}: {}",
                            chunk_index + 1,
                            e
                        ),
                    );
                }
            }
        }

        add_log(logs, "INFO", "RAG", "All chunks processed successfully");

        // Compute and store document quality metrics
        add_log(logs, "INFO", "RAG", "Computing document quality metrics...");

        // Calculate average chunk quality
        let chunk_count = chunks.len() as i64;
        let total_quality: f64 = chunks
            .iter()
            .filter_map(|c| c.quality_score.map(|q| q as f64))
            .sum();
        let avg_quality = if chunk_count > 0 && total_quality > 0.0 {
            Some(total_quality / chunk_count as f64)
        } else {
            None
        };

        // Update document with quality metrics
        self.rag_repository
            .update_document_quality(document.id, avg_quality, None, chunk_count, 0)
            .await
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to update document quality: {}", e),
                );
                AppError::Internal(format!("Failed to update document quality: {}", e))
            })?;

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Document quality: {:.2}", avg_quality.unwrap_or(0.0)),
        );

        add_log(logs, "INFO", "RAG", "Import completed successfully");

        Ok(())
    }

    fn parse_pdf(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;
        use lopdf::Document;

        let document = Document::load(file_path)
            .map_err(|e| AppError::Internal(format!("Failed to load PDF: {}", e)))?;

        let mut page_contents: Vec<PageContent> = Vec::new();
        let mut total_pages = 0i64;
        let mut has_text = false;

        // Extract text per page to preserve page boundaries
        for (page_num, (page_id, _)) in document.get_pages() {
            total_pages += 1;
            match document.extract_text(&[page_id]) {
                Ok(page_text) => {
                    let trimmed = page_text.trim();
                    if !trimmed.is_empty() {
                        has_text = true;
                        page_contents.push(PageContent {
                            page_number: page_num as i64,
                            content: trimmed.to_string(),
                        });
                    }
                }
                Err(_) => {}
            }
        }

        // If no text extracted (scanned PDF), fall back to OCR
        if !has_text {
            add_log(logs, "INFO", "RAG", "No text layer found, running OCR...");
            if let Some(ocr_pages) = self.ocr_pdf_with_grayscale(file_path, logs) {
                return Ok((ParsedContent::Pages(ocr_pages), total_pages, None));
            }
        }

        if page_contents.is_empty() {
            Ok((ParsedContent::Plain(None), total_pages, None))
        } else {
            Ok((ParsedContent::Pages(page_contents), total_pages, None))
        }
    }

    /// OCR PDF pages
    fn ocr_pdf_with_grayscale(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        use crate::interfaces::http::add_log;

        let pdftoppm_cmd = std::env::var("PDFTOPPM_CMD").unwrap_or_else(|_| "pdftoppm".to_string());
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Rasterizing PDF with {} for OCR", pdftoppm_cmd),
        );

        // Create temp directory for OCR processing
        let output_dir =
            std::env::temp_dir().join(format!("gadogado-ocr-{}", uuid::Uuid::new_v4()));
        if let Err(err) = fs::create_dir_all(&output_dir) {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to create OCR temp dir: {}", err),
            );
            return None;
        }

        // Convert PDF to PNG images at 300 DPI
        let output_prefix = output_dir.join("page");
        let output = Command::new(&pdftoppm_cmd)
            .arg("-png")
            .arg("-r")
            .arg("300")
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
                let _ = fs::remove_dir_all(&output_dir);
                return self.ocr_pdf_with_tesseract_fallback(file_path, logs);
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
            let _ = fs::remove_dir_all(&output_dir);
            return None;
        }

        // Get list of generated PNG files
        let mut images: Vec<_> = match fs::read_dir(&output_dir) {
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
                let _ = fs::remove_dir_all(&output_dir);
                return None;
            }
        };

        images.sort();
        if images.is_empty() {
            add_log(logs, "WARN", "RAG", "pdftoppm produced no images");
            let _ = fs::remove_dir_all(&output_dir);
            return None;
        }

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

            // OCR the original image directly (no grayscale preprocessing)
            if let Some(text) = self.ocr_single_image(image_path, logs) {
                if !text.trim().is_empty() {
                    page_contents.push(PageContent {
                        page_number,
                        content: text.trim().to_string(),
                    });
                }
            }
        }

        // Cleanup temp directory
        let _ = fs::remove_dir_all(&output_dir);

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

    /// OCR a single image file with Tesseract
    /// Automatically applies preprocessing if the image has poor contrast
    fn ocr_single_image(
        &self,
        image_path: &Path,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        // Check if image needs preprocessing
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

        let tesseract_cmd =
            std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
        let mut command = Command::new(&tesseract_cmd);
        if let Ok(tessdata_prefix) = std::env::var("TESSDATA_PREFIX") {
            command.env("TESSDATA_PREFIX", tessdata_prefix);
        }
        let output = command
            .arg(ocr_path.as_os_str())
            .arg("stdout")
            .arg("-l")
            .arg("eng+ind")
            .output();

        // Clean up preprocessed image
        if let Some(ref preprocessed) = preprocessed_path {
            let _ = fs::remove_file(preprocessed);
        }

        match output {
            Ok(output) => {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    Some(text)
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

    /// Fallback OCR without grayscale (legacy method)
    fn ocr_pdf_with_tesseract_fallback(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        self.ocr_pdf_with_tesseract_fallback_with_lang(file_path, "eng+ind", logs)
    }

    fn ocr_pdf_with_tesseract_fallback_with_lang(
        &self,
        file_path: &str,
        languages: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<Vec<PageContent>> {
        use crate::interfaces::http::add_log;

        add_log(logs, "INFO", "RAG", "Trying direct Tesseract OCR on PDF...");

        let tesseract_cmd =
            std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
        let mut command = Command::new(&tesseract_cmd);
        if let Ok(tessdata_prefix) = std::env::var("TESSDATA_PREFIX") {
            command.env("TESSDATA_PREFIX", tessdata_prefix);
        }
        let output = command
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
                        // Return as single page since we can't determine page boundaries
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

    fn ocr_pdf_with_tesseract(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        add_log(logs, "INFO", "RAG", "Running OCR with Tesseract...");

        if let Some(text) = self.ocr_pdf_with_pdftoppm(file_path, logs) {
            return Some(text);
        }

        let tesseract_cmd =
            std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
        let mut command = Command::new(&tesseract_cmd);
        if let Ok(tessdata_prefix) = std::env::var("TESSDATA_PREFIX") {
            command.env("TESSDATA_PREFIX", tessdata_prefix);
        }
        let output = command
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

    fn ocr_pdf_with_pdftoppm(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<String> {
        use crate::interfaces::http::add_log;

        let pdftoppm_cmd = std::env::var("PDFTOPPM_CMD").unwrap_or_else(|_| "pdftoppm".to_string());
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Rasterizing PDF with {}", pdftoppm_cmd),
        );
        let output_dir =
            std::env::temp_dir().join(format!("gadogado-ocr-{}", uuid::Uuid::new_v4()));
        if let Err(err) = fs::create_dir_all(&output_dir) {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to create OCR temp dir: {}", err),
            );
            return None;
        }

        let output_prefix = output_dir.join("page");
        let output = Command::new(&pdftoppm_cmd)
            .arg("-png")
            .arg("-r")
            .arg("300")
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
                let _ = fs::remove_dir_all(&output_dir);
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
            let _ = fs::remove_dir_all(&output_dir);
            return None;
        }

        let mut images: Vec<_> = match fs::read_dir(&output_dir) {
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
                let _ = fs::remove_dir_all(&output_dir);
                return None;
            }
        };

        images.sort();
        if images.is_empty() {
            add_log(logs, "WARN", "RAG", "pdftoppm produced no images");
            let _ = fs::remove_dir_all(&output_dir);
            return None;
        }

        let mut combined = String::new();
        for image_path in images {
            let tesseract_cmd =
                std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
            let mut command = Command::new(&tesseract_cmd);
            if let Ok(tessdata_prefix) = std::env::var("TESSDATA_PREFIX") {
                command.env("TESSDATA_PREFIX", tessdata_prefix);
            }
            let output = command
                .arg(image_path.as_os_str())
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
                    continue;
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
                continue;
            }

            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                combined.push_str(text.trim());
                combined.push('\n');
            }
        }

        let _ = fs::remove_dir_all(&output_dir);
        if combined.trim().is_empty() {
            add_log(logs, "WARN", "RAG", "OCR produced empty text");
            None
        } else {
            add_log(logs, "INFO", "RAG", "OCR text extracted successfully");
            Some(combined.trim().to_string())
        }
    }

    fn parse_docx(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;

        let file_bytes = fs::read(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to read DOCX file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to read DOCX file: {}", e))
        })?;
        let docx = docx_rs::read_docx(&file_bytes).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to parse DOCX file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to parse DOCX file: {}", e))
        })?;
        let text = self.extract_docx_text(&docx);

        if text.trim().is_empty() {
            Ok((ParsedContent::Plain(None), 1, None))
        } else {
            // DOCX doesn't have reliable page boundary detection, treat as single page
            Ok((
                ParsedContent::Pages(vec![PageContent {
                    page_number: 1,
                    content: text.trim().to_string(),
                }]),
                1,
                None,
            ))
        }
    }

    fn extract_docx_text(&self, docx: &docx_rs::Docx) -> String {
        let mut lines = Vec::new();
        for child in &docx.document.children {
            self.extract_docx_document_child(child, &mut lines);
        }
        lines.join("\n")
    }

    fn extract_docx_document_child(&self, child: &docx_rs::DocumentChild, lines: &mut Vec<String>) {
        match child {
            docx_rs::DocumentChild::Paragraph(paragraph) => {
                let text = self.extract_docx_paragraph(paragraph);
                if !text.trim().is_empty() {
                    lines.push(text);
                }
            }
            docx_rs::DocumentChild::Table(table) => {
                self.extract_docx_table(table, lines);
            }
            _ => {}
        }
    }

    fn extract_docx_paragraph(&self, paragraph: &docx_rs::Paragraph) -> String {
        let mut buffer = String::new();
        for child in &paragraph.children {
            self.extract_docx_paragraph_child(child, &mut buffer);
        }
        buffer
    }

    fn extract_docx_paragraph_child(&self, child: &docx_rs::ParagraphChild, buffer: &mut String) {
        match child {
            docx_rs::ParagraphChild::Run(run) => {
                self.extract_docx_run(run, buffer);
            }
            docx_rs::ParagraphChild::Hyperlink(link) => {
                for link_child in &link.children {
                    self.extract_docx_paragraph_child(link_child, buffer);
                }
            }
            docx_rs::ParagraphChild::Insert(insert) => {
                for insert_child in &insert.children {
                    if let docx_rs::InsertChild::Run(run) = insert_child {
                        self.extract_docx_run(run, buffer);
                    }
                }
            }
            _ => {}
        }
    }

    fn extract_docx_run(&self, run: &docx_rs::Run, buffer: &mut String) {
        for child in &run.children {
            match child {
                docx_rs::RunChild::Text(text) => buffer.push_str(&text.text),
                docx_rs::RunChild::InstrTextString(text) => buffer.push_str(text),
                docx_rs::RunChild::Tab(_) | docx_rs::RunChild::PTab(_) => buffer.push('\t'),
                docx_rs::RunChild::Break(_) => buffer.push('\n'),
                docx_rs::RunChild::Sym(sym) => buffer.push_str(&sym.char),
                _ => {}
            }
        }
    }

    fn extract_docx_table(&self, table: &docx_rs::Table, lines: &mut Vec<String>) {
        for row in &table.rows {
            let docx_rs::TableChild::TableRow(row) = row;
            let row_text = self.extract_docx_table_row(row);
            if !row_text.trim().is_empty() {
                lines.push(row_text);
            }
        }
    }

    fn extract_docx_table_row(&self, row: &docx_rs::TableRow) -> String {
        let mut cells = Vec::new();
        for cell in &row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = cell;
            let text = self.extract_docx_table_cell(cell);
            if !text.trim().is_empty() {
                cells.push(text);
            }
        }
        cells.join(" | ")
    }

    fn extract_docx_table_cell(&self, cell: &docx_rs::TableCell) -> String {
        let mut parts = Vec::new();
        for content in &cell.children {
            match content {
                docx_rs::TableCellContent::Paragraph(paragraph) => {
                    let text = self.extract_docx_paragraph(paragraph);
                    if !text.trim().is_empty() {
                        parts.push(text);
                    }
                }
                docx_rs::TableCellContent::Table(table) => {
                    let mut nested_lines = Vec::new();
                    self.extract_docx_table(table, &mut nested_lines);
                    if !nested_lines.is_empty() {
                        parts.push(nested_lines.join(" "));
                    }
                }
                _ => {}
            }
        }
        parts.join(" ")
    }

    fn parse_xlsx(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;
        use calamine::{open_workbook, DataType, Reader, Xlsx};

        let mut workbook: Xlsx<_> = open_workbook(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to open Excel file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to open Excel file: {}", e))
        })?;

        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| {
                add_log(logs, "ERROR", "RAG", "No worksheet found in Excel file");
                AppError::Internal("No worksheet found".to_string())
            })?
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to read Excel range {}: {}", file_path, e),
                );
                AppError::Internal(format!("Failed to read Excel range: {}", e))
            })?;

        let mut rows = Vec::new();
        let mut text_lines = Vec::new();
        for row in range.rows() {
            let row_data: Vec<String> = row
                .iter()
                .map(|cell| {
                    cell.as_string()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{}", cell))
                })
                .collect();
            let trimmed_cells: Vec<&str> = row_data
                .iter()
                .map(|cell| cell.trim())
                .filter(|cell| !cell.is_empty())
                .collect();
            if !trimmed_cells.is_empty() {
                text_lines.push(trimmed_cells.join(" | "));
            }
            rows.push(row_data);
        }

        // Excel is single-page, use Plain content type
        let content = if text_lines.is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(text_lines.join("\n")))
        };

        Ok((content, 1, Some(rows)))
    }

    fn parse_csv(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;
        use std::io::BufRead;

        add_log(logs, "INFO", "RAG", "Parsing CSV file...");

        let file = fs::File::open(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to open CSV file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to open CSV file: {}", e))
        })?;

        let reader = std::io::BufReader::new(file);
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut text_lines: Vec<String> = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = match line_result {
                Ok(l) => l,
                Err(e) => {
                    add_log(
                        logs,
                        "WARN",
                        "RAG",
                        &format!("Failed to read line {}: {}", line_num + 1, e),
                    );
                    continue;
                }
            };

            // Simple CSV parsing (handles basic comma-separated values)
            // For complex CSVs with quoted fields, consider using the csv crate
            let row_data: Vec<String> = self.parse_csv_line(&line);

            let trimmed_cells: Vec<&str> = row_data
                .iter()
                .map(|cell| cell.trim())
                .filter(|cell| !cell.is_empty())
                .collect();

            if !trimmed_cells.is_empty() {
                text_lines.push(trimmed_cells.join(" | "));
            }
            rows.push(row_data);
        }

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Parsed {} rows from CSV", rows.len()),
        );

        // CSV is single-page, similar to Excel
        let content = if text_lines.is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(text_lines.join("\n")))
        };

        Ok((content, 1, Some(rows)))
    }

    /// Parse a single CSV line, handling quoted fields
    fn parse_csv_line(&self, line: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_field = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_quotes {
                        // Check for escaped quote ("")
                        if chars.peek() == Some(&'"') {
                            current_field.push('"');
                            chars.next();
                        } else {
                            in_quotes = false;
                        }
                    } else {
                        in_quotes = true;
                    }
                }
                ',' if !in_quotes => {
                    result.push(current_field.trim().to_string());
                    current_field = String::new();
                }
                _ => {
                    current_field.push(c);
                }
            }
        }

        // Don't forget the last field
        result.push(current_field.trim().to_string());
        result
    }

    fn parse_txt(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;

        let text = std::fs::read_to_string(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to read TXT file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to read TXT file: {}", e))
        })?;

        // TXT files are single-page, use Plain content type
        let content = if text.trim().is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(text.trim().to_string()))
        };

        Ok((content, 1, None))
    }

    async fn parse_web(
        &self,
        url: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        self.parse_web_with_options(url, logs, 10, 2).await
    }

    async fn parse_web_with_options(
        &self,
        url: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
        max_pages: usize,
        max_depth: usize,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;

        let crawler = WebCrawler::new(max_pages, max_depth);

        add_log(logs, "INFO", "RAG", &format!("Crawling web site: {}", url));

        let pages = crawler
            .crawl_site(url, std::sync::Arc::clone(logs))
            .await
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to crawl site: {}", e),
                );
                AppError::Internal(format!("Failed to crawl site: {}", e))
            })?;

        let mut combined_content = String::new();
        for page in &pages {
            combined_content.push_str(&format!("\n--- Page: {} ---\n", page.url));
            combined_content.push_str(&format!("Title: {}\n", page.title));
            combined_content.push_str(&format!("Content:\n{}\n", page.content));
        }

        // Web pages are treated as plain text (no page structure)
        let content = if combined_content.trim().is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(combined_content.trim().to_string()))
        };

        Ok((content, pages.len() as i64, None))
    }

    /// Ingest a web page using screenshot OCR mode (Playwright + Tesseract)
    pub async fn ingest_web_ocr(
        &self,
        url: &str,
        collection_id: Option<i64>,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<RagDocument> {
        use crate::interfaces::http::add_log;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Starting OCR web capture: {}", url),
        );

        // Get the script path from resources
        let script_path = self.get_playwright_script_path()?;
        let temp_dir = std::env::temp_dir().join("gadogado_web_ocr");

        // Ensure temp directory exists
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| AppError::Internal(format!("Failed to create temp directory: {}", e)))?;

        let ocr_capture = WebOcrCapture::new(script_path, temp_dir);

        // Capture and OCR the page
        let result = ocr_capture.capture_url(url, logs.clone()).await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!(
                "OCR capture complete: {} - {} characters",
                result.title,
                result.content.len()
            ),
        );

        // Create document input
        let document_input = RagDocumentInput {
            collection_id,
            file_name: result.title.clone(),
            file_path: Some(url.to_string()),
            file_type: "web_ocr".to_string(),
            language: Some("auto".to_string()),
            total_pages: Some(result.manifest.tiles.len() as i64),
        };

        add_log(&logs, "INFO", "RAG", "Creating document record...");

        let document = self.rag_repository.create_document(&document_input).await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Document created: ID {}", document.id),
        );

        // Chunk the content
        let chunks = self.chunk_engine.chunk_text(&result.content)?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Created {} chunks", chunks.len()),
        );

        // Generate embeddings and store chunks
        add_log(&logs, "INFO", "RAG", "Generating embeddings for chunks...");

        let mut processed_count = 0;
        let total_chunks = chunks.len();

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let embedding = self
                .embedding_service
                .generate_embedding(&chunk.content)
                .await
                .map_err(|e| {
                    add_log(
                        &logs,
                        "WARN",
                        "RAG",
                        &format!(
                            "Failed to generate embedding for chunk {}: {}",
                            chunk_index, e
                        ),
                    );
                    e
                })?;

            let chunk_input = RagDocumentChunkInput {
                doc_id: document.id,
                content: chunk.content.clone(),
                page_number: chunk.page_number,
                page_offset: chunk.page_offset,
                chunk_index: chunk_index as i64,
                token_count: Some(chunk.token_count as i64),
            };

            let created_chunk = self.rag_repository.create_chunk(&chunk_input).await?;

            // Update embedding
            let embedding_bytes = EmbeddingService::embedding_to_bytes(&embedding);
            self.rag_repository
                .update_chunk_embedding(created_chunk.id, &embedding_bytes)
                .await?;

            processed_count += 1;

            if processed_count % 10 == 0 || processed_count == total_chunks {
                add_log(
                    &logs,
                    "INFO",
                    "RAG",
                    &format!("Processed {}/{} chunks", processed_count, total_chunks),
                );
            }
        }

        // Clean up temp files
        if let Err(e) = ocr_capture.cleanup(&result.output_dir) {
            add_log(
                &logs,
                "WARN",
                "RAG",
                &format!("Failed to clean up temp files: {}", e),
            );
        }

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!(
                "Web OCR import complete: {} chunks from {}",
                processed_count, url
            ),
        );

        Ok(document)
    }

    /// Get the path to the playwright-capture.js script
    fn get_playwright_script_path(&self) -> Result<std::path::PathBuf> {
        // Try multiple possible locations
        let possible_paths = vec![
            // Development path
            std::path::PathBuf::from("resources/scripts/playwright-capture.js"),
            // Tauri bundled resources path
            std::path::PathBuf::from("../resources/scripts/playwright-capture.js"),
            // Absolute path in src-tauri
            std::path::PathBuf::from(
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("resources/scripts/playwright-capture.js"),
            ),
        ];

        for path in &possible_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        // If not found in standard locations, try to resolve from executable location
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let bundled_path = exe_dir.join("resources/scripts/playwright-capture.js");
                if bundled_path.exists() {
                    return Ok(bundled_path);
                }
            }
        }

        Err(AppError::Internal(
            "Playwright capture script not found. Ensure playwright-capture.js is in resources/scripts/".to_string(),
        ))
    }

    // ============================================================
    // DOCUMENT QUALITY ANALYSIS
    // ============================================================

    /// Analyze document quality after ingestion
    /// Returns quality metrics for the document and its chunks
    pub async fn analyze_document_quality(
        &self,
        document_id: i64,
    ) -> Result<DocumentQualityAnalysis> {
        // Get document info
        let doc = self.rag_repository.get_document(document_id).await?;

        // Get all chunks for the document (use a high limit to get all)
        let chunks = self.rag_repository.get_chunks(document_id, 10000).await?;

        if chunks.is_empty() {
            return Ok(DocumentQualityAnalysis {
                document_id,
                document_name: doc.file_name,
                total_chunks: 0,
                avg_chunk_quality: 0.0,
                min_chunk_quality: 0.0,
                max_chunk_quality: 0.0,
                low_quality_chunk_count: 0,
                avg_chunk_length: 0,
                total_tokens: 0,
                extraction_quality: ExtractionQuality::Unknown,
                issues: vec!["No chunks created".to_string()],
            });
        }

        // Calculate quality metrics
        let mut total_quality = 0.0;
        let mut min_quality = 1.0f32;
        let mut max_quality = 0.0f32;
        let mut low_quality_count = 0;
        let mut total_length = 0;
        let mut total_tokens = 0i64;

        for chunk in &chunks {
            // Estimate quality based on content characteristics
            let quality = self.estimate_chunk_quality(&chunk.content);
            total_quality += quality;
            if quality < min_quality {
                min_quality = quality;
            }
            if quality > max_quality {
                max_quality = quality;
            }
            if quality < 0.5 {
                low_quality_count += 1;
            }
            total_length += chunk.content.len();
            total_tokens += chunk.token_count.unwrap_or(0);
        }

        let avg_quality = total_quality / chunks.len() as f32;
        let avg_length = total_length / chunks.len();

        // Determine extraction quality
        let extraction_quality = if avg_quality >= 0.8 {
            ExtractionQuality::Excellent
        } else if avg_quality >= 0.6 {
            ExtractionQuality::Good
        } else if avg_quality >= 0.4 {
            ExtractionQuality::Fair
        } else {
            ExtractionQuality::Poor
        };

        // Identify issues
        let mut issues = Vec::new();
        if low_quality_count > chunks.len() / 3 {
            issues.push(format!(
                "{}% of chunks have low quality (< 0.5)",
                low_quality_count * 100 / chunks.len()
            ));
        }
        if avg_length < 100 {
            issues.push("Average chunk length is very short".to_string());
        }
        if avg_length > 2000 {
            issues.push("Average chunk length is very long".to_string());
        }

        Ok(DocumentQualityAnalysis {
            document_id,
            document_name: doc.file_name,
            total_chunks: chunks.len(),
            avg_chunk_quality: avg_quality,
            min_chunk_quality: min_quality,
            max_chunk_quality: max_quality,
            low_quality_chunk_count: low_quality_count,
            avg_chunk_length: avg_length,
            total_tokens: total_tokens as usize,
            extraction_quality,
            issues,
        })
    }

    /// Estimate quality of a chunk based on content characteristics
    fn estimate_chunk_quality(&self, content: &str) -> f32 {
        let mut score = 1.0f32;

        // Check content length
        if content.len() < 50 {
            score *= 0.5;
        } else if content.len() < 100 {
            score *= 0.7;
        }

        // Check for meaningful content (not just numbers/symbols)
        let alpha_ratio = content.chars().filter(|c| c.is_alphabetic()).count() as f32
            / content.len().max(1) as f32;
        if alpha_ratio < 0.3 {
            score *= 0.6;
        }

        // Check for OCR noise patterns (repeated chars, unusual sequences)
        let has_noise =
            content.contains("|||") || content.contains("___") || content.contains("...");
        if has_noise {
            score *= 0.7;
        }

        // Check for sentence structure (has periods, question marks, etc.)
        let has_sentence_end =
            content.contains('.') || content.contains('?') || content.contains('!');
        if !has_sentence_end && content.len() > 100 {
            score *= 0.8;
        }

        // Check for proper capitalization
        let first_char_caps = content
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
        if first_char_caps {
            score *= 1.05;
        }

        // Check for excessive whitespace
        let whitespace_ratio = content.chars().filter(|c| c.is_whitespace()).count() as f32
            / content.len().max(1) as f32;
        if whitespace_ratio > 0.5 {
            score *= 0.7;
        }

        score.clamp(0.0, 1.0)
    }
}

/// Document quality analysis result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentQualityAnalysis {
    pub document_id: i64,
    pub document_name: String,
    pub total_chunks: usize,
    pub avg_chunk_quality: f32,
    pub min_chunk_quality: f32,
    pub max_chunk_quality: f32,
    pub low_quality_chunk_count: usize,
    pub avg_chunk_length: usize,
    pub total_tokens: usize,
    pub extraction_quality: ExtractionQuality,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ExtractionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Unknown,
}
