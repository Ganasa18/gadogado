use crate::application::use_cases::retrieval_service::QueryResult;
use crate::domain::error::Result;

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
7. If multiple sources provide information, cite all relevant sources"#.to_string()
    }
    
    fn build_context(results: &[QueryResult]) -> String {
        if results.is_empty() {
            return "No relevant context found in the collection.".to_string();
        }
        
        let mut context = String::from("Context:\n");
        
        for (idx, result) in results.iter().enumerate() {
            context.push_str(&format!(
                "\n--- Source {} ---\n",
                idx + 1
            ));
            context.push_str(&format!(
                "Type: {}\nID: {}\nContent: {}\n",
                result.source_type,
                result.source_id,
                result.content
            ));
            if let Some(score) = result.score {
                context.push_str(&format!("Relevance Score: {:.2}\n", score));
            }
        }
        
        context
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
        let results = vec![
            QueryResult {
                content: "X is a variable".to_string(),
                source_type: "text_chunk".to_string(),
                source_id: 1,
                score: Some(0.95),
            }
        ];
        
        let prompt = PromptEngine::build_prompt("What is X?", &results).unwrap();
        assert!(prompt.contains("X is a variable"));
        assert!(prompt.contains("Source 1"));
        assert!(prompt.contains("Type: text_chunk"));
        assert!(prompt.contains("ID: 1"));
        assert!(prompt.contains("Relevance Score: 0.95"));
    }
}
