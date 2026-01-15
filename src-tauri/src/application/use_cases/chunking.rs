use crate::domain::error::Result;

pub struct ChunkConfig {
    pub max_chunk_size: usize,
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 500,
            overlap: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub token_count: usize,
}

pub struct ChunkEngine {
    config: ChunkConfig,
}

impl ChunkEngine {
    #[warn(dead_code)]
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    pub fn chunk_text(&self, text: &str) -> Result<Vec<Chunk>> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }

        if text.len() <= self.config.max_chunk_size {
            return Ok(vec![Chunk {
                content: text.to_string(),
                token_count: self.estimate_token_count(text),
            }]);
        }

        let chunks = self.split_with_overlap(text);
        Ok(chunks)
    }

    fn split_with_overlap(&self, text: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let mut end = std::cmp::min(start + self.config.max_chunk_size, chars.len());

            if end < chars.len() {
                end = self.find_sentence_boundary(&chars, start, end);
            }

            let chunk_content: String = chars[start..end].iter().collect();
            let token_count = self.estimate_token_count(&chunk_content);

            chunks.push(Chunk {
                content: chunk_content,
                token_count,
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
