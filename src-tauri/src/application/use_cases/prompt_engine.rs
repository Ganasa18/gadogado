use crate::application::use_cases::retrieval_service::QueryResult;
use crate::domain::error::Result;

/// Truncate text for reranking prompts to avoid token limits
fn truncate_for_rerank(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        // Try to break at a word boundary
        let truncated = &text[..max_chars];
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{}...", truncated)
        }
    }
}

pub struct PromptEngine;

impl PromptEngine {
    pub fn build_prompt(query: &str, results: &[QueryResult]) -> Result<String> {
        let system_rules = Self::get_system_rules();
        let context = Self::build_context(results);

        let prompt = format!(
            "{}\n\n{}\n\nUser Question: {}\n\nAnswer the question using only the context provided above. Cite your sources using [Source: type_id] format.",
            system_rules,
            context,
            query.trim()
        );

        Ok(prompt)
    }

    /// Build a reranking prompt for LLM-based relevance scoring
    pub fn build_reranking_prompt(query: &str, chunks: &[QueryResult]) -> String {
        let mut prompt = String::from(
            r#"You are a relevance scoring assistant. Score how relevant each document is to the query.

For each document, provide a relevance score from 0 to 10:
- 10: Perfectly answers the query
- 7-9: Highly relevant, contains key information
- 4-6: Moderately relevant, contains some useful context
- 1-3: Slightly relevant, tangentially related
- 0: Not relevant at all

Query: "#,
        );
        prompt.push_str(query);
        prompt.push_str("\n\nDocuments to score:\n");

        for (idx, chunk) in chunks.iter().enumerate() {
            prompt.push_str(&format!(
                "\n[DOC {}]: {}\n",
                idx + 1,
                truncate_for_rerank(&chunk.content, 300)
            ));
        }

        prompt.push_str(
            r#"

Respond with ONLY a JSON array of scores in order, like: [8, 5, 3, 9, 2]
Do not include any explanation, just the JSON array."#,
        );

        prompt
    }

    /// Parse reranking scores from LLM response
    pub fn parse_reranking_scores(response: &str) -> Option<Vec<f32>> {
        // Find JSON array in response
        let start = response.find('[')?;
        let end = response.rfind(']')? + 1;
        let json_str = &response[start..end];

        // Parse as array of numbers
        let scores: std::result::Result<Vec<f32>, _> = serde_json::from_str(json_str);
        scores.ok()
    }

    /// Rerank results based on LLM scores
    pub fn apply_reranking_scores(results: &mut [QueryResult], scores: &[f32]) {
        if results.len() != scores.len() {
            return;
        }

        for (result, &score) in results.iter_mut().zip(scores.iter()) {
            // Normalize score to 0-1 range and blend with original score
            let normalized = score / 10.0;
            if let Some(original) = result.score {
                // Weighted blend: 60% rerank score, 40% original
                result.score = Some(normalized * 0.6 + original * 0.4);
            } else {
                result.score = Some(normalized);
            }
        }

        // Sort by new scores
        results.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Build a prompt for conversational RAG with context
    pub fn build_conversational_prompt(
        query: &str,
        results: &[QueryResult],
        conversation_summary: Option<&str>,
        recent_messages: &[(String, String)], // (role, content)
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add conversation context
        if let Some(summary) = conversation_summary {
            prompt.push_str("Previous conversation summary:\n");
            prompt.push_str(summary);
            prompt.push_str("\n\n");
        }

        if !recent_messages.is_empty() {
            prompt.push_str("Recent conversation:\n");
            for (role, content) in recent_messages {
                prompt.push_str(&format!("{}: {}\n", role, content));
            }
            prompt.push_str("\n");
        }

        // Add system rules and context
        prompt.push_str(&Self::get_system_rules());
        prompt.push_str("\n\n");
        prompt.push_str(&Self::build_context(results));
        prompt.push_str("\n\n");

        // Add current query
        prompt.push_str(&format!(
            "Current Question: {}\n\nAnswer based on the context and conversation history. Cite sources.",
            query.trim()
        ));

        Ok(prompt)
    }

    fn get_system_rules() -> String {
        r#"You are a helpful AI assistant that answers questions based on the provided context.

IMPORTANT RULES:
1. Use ONLY the information provided in the context below
2. Do NOT fabricate or make up any information
3. Do NOT cite sources that are not in the context
4. When answering, cite the source of each piece of information using [Source: type_id] format
   - For text chunks: [Source: text_chunk_123]
   - For Excel data: [Source: excel_data_456]
5. If the context doesn't contain enough information to answer the question, say so clearly
6. Be concise and direct in your answers
7. If multiple sources provide information, cite all relevant sources"#
            .to_string()
    }

    fn build_context(results: &[QueryResult]) -> String {
        if results.is_empty() {
            return "No relevant context found in the collection.".to_string();
        }

        let mut context = String::from("Context:\n");

        for (idx, result) in results.iter().enumerate() {
            context.push_str(&format!("\n--- Source {} ---\n", idx + 1));

            // Include document name if available
            if let Some(ref doc_name) = result.doc_name {
                context.push_str(&format!("Document: {}\n", doc_name));
            }

            context.push_str(&format!(
                "Type: {}\nID: {}\n",
                result.source_type, result.source_id
            ));

            // Include page number if available
            if let Some(page_num) = result.page_number {
                context.push_str(&format!("Page: {}\n", page_num));
            }

            context.push_str(&format!("Content: {}\n", result.content));

            if let Some(score) = result.score {
                context.push_str(&format!("Relevance Score: {:.2}\n", score));
            }
        }

        context
    }

    // ============================================================
    // SELF-CORRECTING RAG
    // ============================================================

    /// Build a verification prompt to check if an answer is grounded in the context
    pub fn build_verification_prompt(query: &str, answer: &str, results: &[QueryResult]) -> String {
        let context = Self::build_context(results);

        format!(
            r#"You are a fact-checking assistant. Your job is to verify if the given answer is fully supported by the provided context.

CONTEXT:
{}

QUESTION: {}

ANSWER TO VERIFY:
{}

VERIFICATION TASK:
1. Check if every claim in the answer is supported by the context
2. Check if citations are accurate (sources exist and match claims)
3. Check if any information was fabricated

Respond with a JSON object:
{{
    "is_valid": true/false,
    "confidence": 0.0-1.0,
    "issues": ["list of issues found"],
    "unsupported_claims": ["claims not found in context"],
    "missing_citations": ["claims that should have citations"],
    "fabricated_sources": ["sources cited that don't exist"]
}}

Respond with ONLY the JSON object, no explanation."#,
            context, query, answer
        )
    }

    /// Parse verification result from LLM response
    pub fn parse_verification_result(response: &str) -> Option<VerificationResult> {
        // Find JSON object in response
        let start = response.find('{')?;
        let end = response.rfind('}')? + 1;
        let json_str = &response[start..end];

        serde_json::from_str(json_str).ok()
    }

    /// Build a correction prompt based on verification feedback
    pub fn build_correction_prompt(
        query: &str,
        original_answer: &str,
        verification: &VerificationResult,
        results: &[QueryResult],
    ) -> String {
        let context = Self::build_context(results);
        let issues = verification.issues.join("\n- ");
        let unsupported = verification.unsupported_claims.join("\n- ");

        format!(
            r#"You are a helpful AI assistant. Your previous answer had issues that need correction.

CONTEXT:
{}

QUESTION: {}

PREVIOUS ANSWER:
{}

ISSUES FOUND:
- {}

UNSUPPORTED CLAIMS:
- {}

CORRECTION TASK:
Please provide a corrected answer that:
1. Removes or corrects all unsupported claims
2. Only uses information from the provided context
3. Properly cites all sources using [Source: type_id] format
4. Acknowledges if the context doesn't fully answer the question

Provide your corrected answer:"#,
            context,
            query,
            original_answer,
            if issues.is_empty() { "None" } else { &issues },
            if unsupported.is_empty() {
                "None"
            } else {
                &unsupported
            }
        )
    }

    /// Build a prompt that encourages self-reflection before answering
    pub fn build_reflective_prompt(query: &str, results: &[QueryResult]) -> Result<String> {
        let system_rules = Self::get_system_rules();
        let context = Self::build_context(results);

        let prompt = format!(
            r#"{}

{}

User Question: {}

Before answering, consider:
1. What specific information from the context addresses this question?
2. Are there any gaps in the available information?
3. What sources will you cite for each claim?

Now provide your answer, citing sources for every factual claim:"#,
            system_rules,
            context,
            query.trim()
        );

        Ok(prompt)
    }

    // ============================================================
    // CONVERSATIONAL RESPONSE GENERATION
    // ============================================================

    /// Build a conversational prompt for natural language responses
    pub fn build_conversational_nl_prompt(
        query: &str,
        results: &[QueryResult],
        language: Option<&str>,
    ) -> Result<String> {
        let response_lang_instruction = match language {
            Some("id") | Some("indonesia") | Some("indonesian") => {
                r#"IMPORTANT - Respond in INDONESIAN with a casual, conversational tone:
- Write as if you're chatting with a friend, NOT writing a formal report
- Use casual language like "aku nemu", "gue dapet", "kayak gini", "nih"
- Avoid formal words like "berdasarkan", "tersedia", "mengindikasikan"
- Keep it friendly and natural

Examples:
- "Based on the documents, I found 3 results" → "Oke, aku nemu 3 hasil yang cocok nih"
- "No results found" → "Hmm, gak ada yang cocok nih. Mungkin coba kata kunci lain?"
- "Here is the information" → "Nih, infonya aku udah kumpulin"#
            }
            _ => {
                r#"IMPORTANT - Use a casual, conversational tone:
- Write as if you're chatting with a friend, NOT writing a formal report
- Use casual language like "I found", "Here's what I got", "like this"
- Avoid overly formal words
- Keep it friendly and natural

Examples:
- "Based on the documents, I found 3 results" → "Okay, I found 3 results for you"
- "No results found" → "Hmm, couldn't find anything. Maybe try different keywords?"
- "Here is the information" → "Here's what I found for you"#
            }
        };

        let context = Self::build_context(results);

        let prompt = format!(
            r#"You are a friendly and helpful document search assistant with a casual, conversational tone.

{}

CONTEXT RULES:
1. Use ONLY the information provided in the context below
2. Do NOT fabricate or make up any information
3. Cite sources using [Source: type_id] format so users can verify

{}

User Question: {}

Provide a conversational, helpful response:"#,
            response_lang_instruction,
            if results.is_empty() {
                "No relevant documents found.".to_string()
            } else {
                format!("CONTEXT:\n{}", context)
            },
            query.trim()
        );

        Ok(prompt)
    }

    /// Build a conversational prompt with few-shot examples
    pub fn build_conversational_nl_prompt_with_few_shot(
        query: &str,
        results: &[QueryResult],
        language: Option<&str>,
    ) -> Result<String> {
        let response_lang_instruction = match language {
            Some("id") | Some("indonesia") | Some("indonesian") => {
                r#"IMPORTANT - Respond in INDONESIAN with a casual, conversational tone:
- Write as if you're chatting with a friend, NOT writing a formal report
- Use casual language like "aku nemu", "gue dapet", "kayak gini", "nih"
- Avoid formal words like "berdasarkan", "tersedia", "mengindikasikan"
- Keep it friendly and natural

Examples:
- "Based on the documents, I found 3 results" → "Oke, aku nemu 3 hasil yang cocok nih"
- "No results found" → "Hmm, gak ada yang cocok nih. Mungkin coba kata kunci lain?"#
            }
            _ => {
                r#"IMPORTANT - Use a casual, conversational tone:
- Write as if you're chatting with a friend, NOT writing a formal report
- Use casual language like "I found", "Here's what I got", "like this"
- Avoid overly formal words
- Keep it friendly and natural

Examples:
- "Based on the documents, I found 3 results" → "Okay, I found 3 results for you"
- "No results found" → "Hmm, couldn't find anything. Maybe try different keywords?"#
            }
        };

        let few_shot_examples = r#"

EXAMPLE RESPONSES (follow this style):

Example 1 - When finding information:
"Oke, aku nemu beberapa info yang kamu cari. Jadi di [Source: text_chunk_123] disebutin bahwa...

Nah, dari dokumen yang ada, poin-poin pentingnya:
1. ... [Source: excel_data_456]
2. ...

Kira-kira gitu infonya, ada yang mau ditanyain lagi?"

Example 2 - When nothing found:
"Hmm, gak ada yang cocok nih. Mungkin coba kata kunci lain atau lebih spesifik?"

Example 3 - When answering specific questions:
"Jadi gini, menurut [Source: text_chunk_789] ..."#;

        let context = Self::build_context(results);

        let prompt = format!(
            r#"You are a friendly and helpful document search assistant with a casual, conversational tone.

{}{}

CONTEXT RULES:
1. Use ONLY the information provided in the context below
2. Do NOT fabricate or make up any information
3. Cite sources using [Source: type_id] format so users can verify

{}

User Question: {}

Provide a conversational, helpful response following the example style above:"#,
            response_lang_instruction,
            few_shot_examples,
            if results.is_empty() {
                "No relevant documents found.".to_string()
            } else {
                format!("CONTEXT:\n{}", context)
            },
            query.trim()
        );

        Ok(prompt)
    }

    /// Build a conversational prompt with chat history
    pub fn build_conversational_prompt_with_chat(
        query: &str,
        results: &[QueryResult],
        conversation_summary: Option<&str>,
        recent_messages: &[(String, String)],
        language: Option<&str>,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add conversation context
        if let Some(summary) = conversation_summary {
            prompt.push_str("Previous conversation summary:\n");
            prompt.push_str(summary);
            prompt.push_str("\n\n");
        }

        if !recent_messages.is_empty() {
            prompt.push_str("Recent conversation:\n");
            for (role, content) in recent_messages {
                prompt.push_str(&format!("{}: {}\n", role, content));
            }
            prompt.push_str("\n");
        }

        // Add conversational system rules
        let conversational_rules = match language {
            Some("id") | Some("indonesia") | Some("indonesian") => {
                r#"Kamu adalah asisten pencarian dokumen yang ramah dengan gaya bicara santai.

ATURAN PENTING:
1. Gunakan HANYA informasi dari konteks yang disediakan
2. JANGAN mengarang atau membuat informasi
3. JANGAN mencantumkan sumber yang tidak ada di konteks
4. Saat menjawab, cantumkan sumber setiap informasi menggunakan format [Source: type_id]
5. Jika konteks tidak punya cukup info, katakan dengan jujur
6. Gunakan bahasa SANTAI seperti ngobrol sama teman
   - "Berdasarkan dokumen" → "Oke, aku nemu..."
   - "Tidak ditemukan" → "Hmm, gak ada yang cocok nih"
   - "Berikut informasinya" → "Nih, infonya...""#
            }
            _ => {
                r#"You are a friendly document search assistant with a casual, conversational tone.

IMPORTANT RULES:
1. Use ONLY the information provided in the context below
2. Do NOT fabricate or make up any information
3. Do NOT cite sources that are not in the context
4. When answering, cite the source of each piece of information using [Source: type_id] format
5. If the context doesn't contain enough information to answer the question, say so clearly
6. Use CASUAL language like you're chatting with a friend
   - "Based on the documents" → "Okay, I found..."
   - "No results found" → "Hmm, couldn't find anything"
   - "Here is the information" → "Here's what I got"#
            }
        };

        prompt.push_str(conversational_rules);
        prompt.push_str("\n\n");
        prompt.push_str(&Self::build_context(results));
        prompt.push_str("\n\n");

        // Add current query
        prompt.push_str(&format!(
            "Current Question: {}\n\nAnswer based on the context and conversation history. Use a casual, conversational tone. Cite sources.",
            query.trim()
        ));

        Ok(prompt)
    }
}

// ============================================================
// VERIFICATION TYPES
// ============================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub confidence: f32,
    #[serde(default)]
    pub issues: Vec<String>,
    #[serde(default)]
    pub unsupported_claims: Vec<String>,
    #[serde(default)]
    pub missing_citations: Vec<String>,
    #[serde(default)]
    pub fabricated_sources: Vec<String>,
}

impl VerificationResult {
    /// Check if the answer needs correction
    pub fn needs_correction(&self) -> bool {
        !self.is_valid
            || self.confidence < 0.7
            || !self.unsupported_claims.is_empty()
            || !self.fabricated_sources.is_empty()
    }

    /// Get a summary of issues for logging
    pub fn summary(&self) -> String {
        if self.is_valid && self.issues.is_empty() {
            "Answer verified successfully".to_string()
        } else {
            format!(
                "Issues: {} unsupported claims, {} missing citations, {} fabricated sources",
                self.unsupported_claims.len(),
                self.missing_citations.len(),
                self.fabricated_sources.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_no_results() {
        let prompt = PromptEngine::build_prompt("What is X?", &[]).unwrap();
        assert!(prompt.contains("No relevant context found"));
        assert!(prompt.contains("User Question: What is X?"));
    }

    #[test]
    fn test_build_prompt_with_results() {
        let results = vec![QueryResult {
            content: "X is a variable".to_string(),
            source_type: "text_chunk".to_string(),
            source_id: 1,
            score: Some(0.95),
            page_number: Some(3),
            page_offset: Some(150),
            doc_name: Some("test.pdf".to_string()),
        }];

        let prompt = PromptEngine::build_prompt("What is X?", &results).unwrap();
        assert!(prompt.contains("X is a variable"));
        assert!(prompt.contains("Source 1"));
        assert!(prompt.contains("Type: text_chunk"));
        assert!(prompt.contains("ID: 1"));
        assert!(prompt.contains("Relevance Score: 0.95"));
        assert!(prompt.contains("Document: test.pdf"));
        assert!(prompt.contains("Page: 3"));
    }
}
