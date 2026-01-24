use super::super::{AppError, ParseResult, ParsedContent, RagIngestionUseCase};

use crate::application::use_cases::chunking::PageContent;

use std::fs;

impl RagIngestionUseCase {
    pub(in crate::application::use_cases::rag_ingestion) fn parse_docx(
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
}
