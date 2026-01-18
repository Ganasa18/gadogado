use crate::domain::error::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkStrategy {
    /// Fixed size chunking with overlap (original behavior)
    FixedSize,
    /// Content-aware chunking that respects document structure
    ContentAware,
    /// Semantic chunking based on paragraph boundaries
    Semantic,
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        ChunkStrategy::ContentAware
    }
}

pub struct ChunkConfig {
    pub max_chunk_size: usize,
    pub overlap: usize,
    pub strategy: ChunkStrategy,
    /// Minimum chunk size (avoid tiny chunks)
    pub min_chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 500,
            overlap: 50,
            strategy: ChunkStrategy::ContentAware,
            min_chunk_size: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub token_count: usize,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    /// Quality score for the chunk (0.0 to 1.0)
    pub quality_score: Option<f32>,
    /// Type of content (header, paragraph, list, code, table)
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PageContent {
    pub page_number: i64,
    pub content: String,
}

/// Detected content block with type information
#[derive(Debug, Clone)]
struct ContentBlock {
    content: String,
    block_type: ContentBlockType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ContentBlockType {
    Header,
    Paragraph,
    List,
    Code,
    Table,
    Unknown,
}

pub struct ChunkEngine {
    config: ChunkConfig,
}

impl ChunkEngine {
    #[allow(dead_code)]
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    /// Chunk plain text without page metadata (legacy method for backward compatibility)
    pub fn chunk_text(&self, text: &str) -> Result<Vec<Chunk>> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // Use content-aware chunking if enabled
        match self.config.strategy {
            ChunkStrategy::ContentAware => self.chunk_text_content_aware(text),
            ChunkStrategy::Semantic => self.chunk_text_semantic(text),
            ChunkStrategy::FixedSize => self.chunk_text_fixed(text),
        }
    }

    /// Fixed-size chunking (original behavior)
    fn chunk_text_fixed(&self, text: &str) -> Result<Vec<Chunk>> {
        if text.len() <= self.config.max_chunk_size {
            return Ok(vec![Chunk {
                content: text.to_string(),
                token_count: self.estimate_token_count(text),
                page_number: None,
                page_offset: None,
                quality_score: Some(self.calculate_quality_score(text)),
                content_type: None,
            }]);
        }

        let chunks = self.split_with_overlap(text, None, 0);
        Ok(chunks)
    }

    /// Content-aware chunking that respects document structure
    fn chunk_text_content_aware(&self, text: &str) -> Result<Vec<Chunk>> {
        let blocks = self.detect_content_blocks(text);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_type: Option<ContentBlockType> = None;

        for block in blocks {
            // If adding this block would exceed max size, flush current chunk
            if !current_chunk.is_empty()
                && current_chunk.len() + block.content.len() > self.config.max_chunk_size
            {
                chunks.push(self.create_chunk_from_content(&current_chunk, None, 0, current_type));

                // Keep overlap from previous chunk
                let overlap_start = current_chunk.len().saturating_sub(self.config.overlap);
                current_chunk = current_chunk[overlap_start..].to_string();
            }

            // If block itself is too large, split it
            if block.content.len() > self.config.max_chunk_size {
                // Flush any pending content first
                if !current_chunk.is_empty() {
                    chunks.push(self.create_chunk_from_content(
                        &current_chunk,
                        None,
                        0,
                        current_type,
                    ));
                    current_chunk.clear();
                }

                // Split large block
                let sub_chunks = self.split_with_overlap(&block.content, None, 0);
                for mut sub_chunk in sub_chunks {
                    sub_chunk.content_type = Some(self.block_type_to_string(block.block_type));
                    chunks.push(sub_chunk);
                }
            } else {
                // Add block to current chunk
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(&block.content);
                current_type = Some(block.block_type);
            }
        }

        // Flush remaining content
        if !current_chunk.is_empty() && current_chunk.len() >= self.config.min_chunk_size {
            chunks.push(self.create_chunk_from_content(&current_chunk, None, 0, current_type));
        } else if !current_chunk.is_empty() && !chunks.is_empty() {
            // Append small remaining content to last chunk
            if let Some(last) = chunks.last_mut() {
                last.content.push_str("\n\n");
                last.content.push_str(&current_chunk);
                last.token_count = self.estimate_token_count(&last.content);
                last.quality_score = Some(self.calculate_quality_score(&last.content));
            }
        } else if !current_chunk.is_empty() {
            // Only chunk, even if small
            chunks.push(self.create_chunk_from_content(&current_chunk, None, 0, current_type));
        }

        Ok(chunks)
    }

    /// Semantic chunking based on paragraph boundaries
    fn chunk_text_semantic(&self, text: &str) -> Result<Vec<Chunk>> {
        // Split by double newlines (paragraphs)
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for para in paragraphs {
            if !current_chunk.is_empty()
                && current_chunk.len() + para.len() + 2 > self.config.max_chunk_size
            {
                chunks.push(self.create_chunk_from_content(
                    &current_chunk,
                    None,
                    0,
                    Some(ContentBlockType::Paragraph),
                ));
                current_chunk.clear();
            }

            if para.len() > self.config.max_chunk_size {
                // Split large paragraph
                if !current_chunk.is_empty() {
                    chunks.push(self.create_chunk_from_content(
                        &current_chunk,
                        None,
                        0,
                        Some(ContentBlockType::Paragraph),
                    ));
                    current_chunk.clear();
                }
                let sub_chunks = self.split_with_overlap(para, None, 0);
                chunks.extend(sub_chunks);
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(self.create_chunk_from_content(
                &current_chunk,
                None,
                0,
                Some(ContentBlockType::Paragraph),
            ));
        }

        Ok(chunks)
    }

    /// Detect content blocks in text (headers, lists, code, paragraphs)
    fn detect_content_blocks(&self, text: &str) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let mut current_block = String::new();
        let mut current_type = ContentBlockType::Unknown;

        for line in lines.iter() {
            let line_type = self.detect_line_type(line);

            // Check if we need to start a new block
            let should_break = match (&current_type, &line_type) {
                (_, ContentBlockType::Header) => true,
                (ContentBlockType::Header, _) => true,
                (ContentBlockType::List, t) if *t != ContentBlockType::List => true,
                (ContentBlockType::Code, t) if *t != ContentBlockType::Code => true,
                (ContentBlockType::Table, t) if *t != ContentBlockType::Table => true,
                (t, ContentBlockType::List) if *t != ContentBlockType::List => true,
                (t, ContentBlockType::Code) if *t != ContentBlockType::Code => true,
                (t, ContentBlockType::Table) if *t != ContentBlockType::Table => true,
                _ => line.trim().is_empty() && !current_block.is_empty(),
            };

            if should_break && !current_block.is_empty() {
                blocks.push(ContentBlock {
                    content: current_block.trim().to_string(),
                    block_type: current_type,
                });
                current_block.clear();
            }

            if !line.trim().is_empty() {
                if current_block.is_empty() {
                    current_type = line_type;
                }
                if !current_block.is_empty() {
                    current_block.push('\n');
                }
                current_block.push_str(line);
            }
        }

        // Add final block
        if !current_block.is_empty() {
            blocks.push(ContentBlock {
                content: current_block.trim().to_string(),
                block_type: current_type,
            });
        }

        blocks
    }

    /// Detect the type of a single line
    fn detect_line_type(&self, line: &str) -> ContentBlockType {
        let trimmed = line.trim();

        // Headers (Markdown style or all caps short lines)
        if trimmed.starts_with('#') {
            return ContentBlockType::Header;
        }
        if trimmed.len() < 100
            && trimmed
                .chars()
                .all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_punctuation())
            && trimmed.len() > 3
        {
            return ContentBlockType::Header;
        }

        // List items
        if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('•') {
            return ContentBlockType::List;
        }
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.contains('.')
            && trimmed.len() > 2
        {
            let after_num: String = trimmed.chars().skip_while(|c| c.is_ascii_digit()).collect();
            if after_num.starts_with('.') || after_num.starts_with(')') {
                return ContentBlockType::List;
            }
        }

        // Code blocks (indented or markdown fenced)
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            return ContentBlockType::Code;
        }
        if line.starts_with("    ") || line.starts_with("\t") {
            return ContentBlockType::Code;
        }

        // Table (pipe-separated or tab-separated with multiple columns)
        if trimmed.contains('|') && trimmed.matches('|').count() >= 2 {
            return ContentBlockType::Table;
        }

        ContentBlockType::Paragraph
    }

    fn block_type_to_string(&self, block_type: ContentBlockType) -> String {
        match block_type {
            ContentBlockType::Header => "header".to_string(),
            ContentBlockType::Paragraph => "paragraph".to_string(),
            ContentBlockType::List => "list".to_string(),
            ContentBlockType::Code => "code".to_string(),
            ContentBlockType::Table => "table".to_string(),
            ContentBlockType::Unknown => "unknown".to_string(),
        }
    }

    fn create_chunk_from_content(
        &self,
        content: &str,
        page_number: Option<i64>,
        page_offset: i64,
        content_type: Option<ContentBlockType>,
    ) -> Chunk {
        let trimmed = content.trim();
        Chunk {
            content: trimmed.to_string(),
            token_count: self.estimate_token_count(trimmed),
            page_number,
            page_offset: Some(page_offset),
            quality_score: Some(self.calculate_quality_score(trimmed)),
            content_type: content_type.map(|t| self.block_type_to_string(t)),
        }
    }

    /// Calculate a quality score for a chunk (0.0 to 1.0)
    fn calculate_quality_score(&self, text: &str) -> f32 {
        let mut score = 1.0f32;

        // Penalize very short chunks
        if text.len() < 50 {
            score *= 0.5;
        } else if text.len() < 100 {
            score *= 0.8;
        }

        // Penalize chunks that are mostly punctuation/symbols
        let alpha_ratio =
            text.chars().filter(|c| c.is_alphabetic()).count() as f32 / text.len().max(1) as f32;
        if alpha_ratio < 0.5 {
            score *= 0.7;
        }

        // Penalize chunks that are mostly numbers
        let digit_ratio =
            text.chars().filter(|c| c.is_ascii_digit()).count() as f32 / text.len().max(1) as f32;
        if digit_ratio > 0.5 {
            score *= 0.8;
        }

        // Reward chunks with complete sentences
        if text.ends_with('.') || text.ends_with('!') || text.ends_with('?') {
            score *= 1.1;
        }

        // Reward chunks with proper capitalization
        if text
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            score *= 1.05;
        }

        score.clamp(0.0, 1.0)
    }

    /// Chunk text with page metadata - processes pages sequentially and tracks page boundaries
    /// Chunks that span page boundaries are assigned to the starting page
    pub fn chunk_pages(&self, pages: &[PageContent]) -> Result<Vec<Chunk>> {
        if pages.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_chunks = Vec::new();

        // Build a combined text with page boundary markers for tracking
        // Each page boundary is tracked by accumulating character offsets
        let mut combined_text = String::new();
        let mut page_boundaries: Vec<(usize, i64)> = Vec::new(); // (char_offset, page_number)

        for page in pages {
            let trimmed = page.content.trim();
            if trimmed.is_empty() {
                continue;
            }

            page_boundaries.push((combined_text.len(), page.page_number));
            combined_text.push_str(trimmed);
            combined_text.push('\n'); // Separate pages with newline
        }

        if combined_text.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Chunk the combined text
        let chars: Vec<char> = combined_text.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let mut end = std::cmp::min(start + self.config.max_chunk_size, chars.len());

            if end < chars.len() {
                end = self.find_sentence_boundary(&chars, start, end);
            }

            let chunk_content: String = chars[start..end].iter().collect();
            let trimmed_chunk = chunk_content.trim();

            if !trimmed_chunk.is_empty() {
                let token_count = self.estimate_token_count(trimmed_chunk);

                // Find which page this chunk starts on
                let (page_number, page_offset) = self.find_page_for_offset(&page_boundaries, start);

                all_chunks.push(Chunk {
                    content: trimmed_chunk.to_string(),
                    token_count,
                    page_number: Some(page_number),
                    page_offset: Some(page_offset),
                    quality_score: Some(self.calculate_quality_score(trimmed_chunk)),
                    content_type: None,
                });
            }

            if end >= chars.len() {
                break;
            }

            start = end - self.config.overlap;
        }

        Ok(all_chunks)
    }

    /// Find the page number and offset within page for a given character offset
    fn find_page_for_offset(&self, boundaries: &[(usize, i64)], offset: usize) -> (i64, i64) {
        let mut current_page = 1i64;
        let mut page_start = 0usize;

        for (boundary_offset, page_num) in boundaries {
            if offset >= *boundary_offset {
                current_page = *page_num;
                page_start = *boundary_offset;
            } else {
                break;
            }
        }

        let page_offset = (offset - page_start) as i64;
        (current_page, page_offset)
    }

    fn split_with_overlap(
        &self,
        text: &str,
        page_number: Option<i64>,
        base_offset: i64,
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let mut end = std::cmp::min(start + self.config.max_chunk_size, chars.len());

            if end < chars.len() {
                end = self.find_sentence_boundary(&chars, start, end);
            }

            let chunk_content: String = chars[start..end].iter().collect();
            let trimmed = chunk_content.trim();
            let token_count = self.estimate_token_count(trimmed);

            chunks.push(Chunk {
                content: trimmed.to_string(),
                token_count,
                page_number,
                page_offset: Some(base_offset + start as i64),
                quality_score: Some(self.calculate_quality_score(trimmed)),
                content_type: None,
            });

            if end >= chars.len() {
                break;
            }

            start = end - self.config.overlap;
        }

        chunks
    }

    fn find_sentence_boundary(&self, chars: &[char], start: usize, max_end: usize) -> usize {
        let search_start = std::cmp::max(start + 50, max_end - 50);
        let search_end = max_end;

        for i in (search_start..search_end).rev() {
            let c = chars.get(i);
            if let Some(c) = c {
                if *c == '.' || *c == '!' || *c == '?' {
                    return i + 1;
                }
            }
        }

        max_end
    }

    fn estimate_token_count(&self, text: &str) -> usize {
        (text.len() / 4).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text() {
        let engine = ChunkEngine::default();
        let text = "This is a short text.";
        let chunks = engine.chunk_text(text).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }

    #[test]
    fn test_chunk_overlap() {
        let engine = ChunkEngine::default();
        let text = "A".repeat(1000);
        let chunks = engine.chunk_text(&text).unwrap();
        assert!(chunks.len() > 1);
    }
}
