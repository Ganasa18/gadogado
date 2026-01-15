use crate::application::use_cases::chunking::ChunkEngine;
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::web_crawler::WebCrawler;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::rag_entities::{
    RagDocument, RagDocumentChunkInput, RagDocumentInput, RagExcelDataInput,
};
use crate::infrastructure::db::rag::repository::RagRepository;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

type ParseResult = Result<(Option<String>, i64, Option<Vec<Vec<String>>>)>;

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
            "txt" => "txt",
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

        let (content, pages, excel_data) = match file_type {
            "pdf" => self.parse_pdf(file_path, &logs)?,
            "docx" => self.parse_docx(file_path)?,
            "xlsx" => self.parse_xlsx(file_path)?,
            "txt" => self.parse_txt(file_path)?,
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

        if let Some(ref text_content) = content {
            add_log(
                &logs,
                "INFO",
                "RAG",
                &format!("Chunking text ({} chars)...", text_content.len()),
            );

            let chunks = self
                .chunk_engine
                .chunk_text(text_content)
                .map_err(|e| AppError::Internal(format!("Failed to chunk text: {}", e)))?;

            add_log(
                &logs,
                "INFO",
                "RAG",
                &format!("Created {} chunks", chunks.len()),
            );

            for (chunk_index, chunk) in chunks.iter().enumerate() {
                add_log(
                    &logs,
                    "INFO",
                    "RAG",
                    &format!("Processing chunk {}/{}...", chunk_index + 1, chunks.len()),
                );

                let chunk_input = RagDocumentChunkInput {
                    doc_id: document.id,
                    content: chunk.content.clone(),
                    page_number: None,
                    chunk_index: chunk_index as i64,
                    token_count: Some(chunk.token_count as i64),
                };

                let created_chunk = self
                    .rag_repository
                    .create_chunk(&chunk_input)
                    .await
                    .map_err(|e| {
                        add_log(
                            &logs,
                            "ERROR",
                            "RAG",
                            &format!("Failed to store chunk {}: {}", chunk_index + 1, e),
                        );
                        AppError::Internal(format!("Failed to store chunk: {}", e))
                    })?;

                add_log(
                    &logs,
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
                                    &logs,
                                    "ERROR",
                                    "RAG",
                                    &format!(
                                        "Failed to update chunk embedding {}: {}",
                                        chunk_index + 1,
                                        e
                                    ),
                                );
                                AppError::Internal(format!(
                                    "Failed to update chunk embedding: {}",
                                    e
                                ))
                            })?;
                        add_log(
                            &logs,
                            "INFO",
                            "RAG",
                            &format!("Chunk {}/{} processed", chunk_index + 1, chunks.len()),
                        );
                    }
                    Err(e) => {
                        add_log(
                            &logs,
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

            add_log(&logs, "INFO", "RAG", "All chunks processed successfully");
        } else {
            add_log(
                &logs,
                "WARN",
                "RAG",
                &format!(
                    "No text extracted from {} ({}); embeddings skipped",
                    file_name, file_type
                ),
            );
        }

        add_log(&logs, "INFO", "RAG", "Import completed successfully");

        Ok(document)
    }

    fn parse_pdf(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use lopdf::Document;

        let document = Document::load(file_path)
            .map_err(|e| AppError::Internal(format!("Failed to load PDF: {}", e)))?;

        let mut text = String::new();
        let mut pages = 0i64;

        for (_, (page_id, _)) in document.get_pages() {
            match document.extract_text(&[page_id]) {
                Ok(page_text) => {
                    text.push_str(&page_text);
                    text.push('\n');
                }
                Err(_) => {}
            }
            pages += 1;
        }

        let mut text_opt = if text.trim().is_empty() {
            None
        } else {
            Some(text.trim().to_string())
        };

        if text_opt.is_none() {
            text_opt = self.ocr_pdf_with_tesseract(file_path, logs);
        }

        Ok((text_opt, pages, None))
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

        let tesseract_cmd = std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
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
        let output_dir = std::env::temp_dir().join(format!(
            "gadogado-ocr-{}",
            uuid::Uuid::new_v4()
        ));
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
            let tesseract_cmd = std::env::var("TESSERACT_CMD")
                .unwrap_or_else(|_| "tesseract".to_string());
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

    fn parse_docx(&self, file_path: &str) -> ParseResult {
        let file_bytes = fs::read(file_path)
            .map_err(|e| AppError::Internal(format!("Failed to read DOCX file: {}", e)))?;
        let docx = docx_rs::read_docx(&file_bytes)
            .map_err(|e| AppError::Internal(format!("Failed to parse DOCX file: {}", e)))?;
        let text = self.extract_docx_text(&docx);
        let text_opt = if text.trim().is_empty() {
            None
        } else {
            Some(text.trim().to_string())
        };

        Ok((text_opt, 1, None))
    }

    fn extract_docx_text(&self, docx: &docx_rs::Docx) -> String {
        let mut lines = Vec::new();
        for child in &docx.document.children {
            self.extract_docx_document_child(child, &mut lines);
        }
        lines.join("\n")
    }

    fn extract_docx_document_child(
        &self,
        child: &docx_rs::DocumentChild,
        lines: &mut Vec<String>,
    ) {
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

    fn extract_docx_paragraph_child(
        &self,
        child: &docx_rs::ParagraphChild,
        buffer: &mut String,
    ) {
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
            if let docx_rs::TableChild::TableRow(row) = row {
                let row_text = self.extract_docx_table_row(row);
                if !row_text.trim().is_empty() {
                    lines.push(row_text);
                }
            }
        }
    }

    fn extract_docx_table_row(&self, row: &docx_rs::TableRow) -> String {
        let mut cells = Vec::new();
        for cell in &row.cells {
            if let docx_rs::TableRowChild::TableCell(cell) = cell {
                let text = self.extract_docx_table_cell(cell);
                if !text.trim().is_empty() {
                    cells.push(text);
                }
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

    fn parse_xlsx(&self, file_path: &str) -> ParseResult {
        use calamine::{open_workbook, DataType, Reader, Xlsx};

        let mut workbook: Xlsx<_> = open_workbook(file_path)
            .map_err(|e| AppError::Internal(format!("Failed to open Excel file: {}", e)))?;

        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| AppError::Internal("No worksheet found".to_string()))?
            .map_err(|e| AppError::Internal(format!("Failed to read Excel range: {}", e)))?;

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

        let text_opt = if text_lines.is_empty() {
            None
        } else {
            Some(text_lines.join("\n"))
        };

        Ok((text_opt, 1, Some(rows)))
    }

    fn parse_txt(&self, file_path: &str) -> ParseResult {
        let text = std::fs::read_to_string(file_path)
            .map_err(|e| AppError::Internal(format!("Failed to read TXT file: {}", e)))?;

        let text_opt = if text.trim().is_empty() {
            None
        } else {
            Some(text.trim().to_string())
        };

        Ok((text_opt, 1, None))
    }

    async fn parse_web(&self, url: &str, logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>) -> ParseResult {
        use crate::interfaces::http::add_log;
        
        let crawler = WebCrawler::new(10, 2);
        
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Crawling web site: {}", url),
        );

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

        let text_opt = if combined_content.trim().is_empty() {
            None
        } else {
            Some(combined_content.trim().to_string())
        };

        Ok((text_opt, pages.len() as i64, None))
    }
}
