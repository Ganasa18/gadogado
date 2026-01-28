//! Few-Shot Prompt Builder for SQL Query Generation
//!
//! Constructs LLM prompts with template examples to improve query accuracy

use super::template_matcher::TemplateMatch;
use crate::domain::rag_entities::QueryPlan;
use std::fmt::Write;

/// Few-shot prompt builder
pub struct FewShotPromptBuilder {
    max_examples: usize,
    include_schema_info: bool,
}

impl FewShotPromptBuilder {
    /// Create a new prompt builder
    pub fn new(max_examples: usize, include_schema_info: bool) -> Self {
        Self {
            max_examples,
            include_schema_info,
        }
    }

    /// Build a few-shot prompt for SQL query generation
    pub fn build_prompt(
        &self,
        user_query: &str,
        matched_templates: &[TemplateMatch],
        available_tables: &[(String, Vec<String>)], // (table_name, columns)
    ) -> String {
        let mut prompt = String::new();

        // System instruction
        writeln!(
            prompt,
            "You are a SQL query generator. Generate SQL queries based on natural language questions."
        )
        .unwrap();
        writeln!(prompt, "Use the examples below as a guide for query patterns.\n").unwrap();

        // Add few-shot examples
        if !matched_templates.is_empty() {
            writeln!(prompt, "## Examples\n").unwrap();

            for (idx, template_match) in matched_templates
                .iter()
                .enumerate()
                .take(self.max_examples)
            {
                self.add_example(&mut prompt, idx + 1, template_match);
            }

            writeln!(prompt).unwrap();
        }

        // Add schema information
        if self.include_schema_info {
            self.add_schema_info(&mut prompt, available_tables);
        }

        // Add the actual query
        writeln!(prompt, "## Task").unwrap();
        writeln!(prompt, "Generate a SQL query for: {}\n", user_query).unwrap();

        // Add output format instruction
        writeln!(prompt, "## Output Format").unwrap();
        writeln!(
            prompt,
            "Provide only the SQL query without explanation or markdown formatting."
        )
        .unwrap();

        prompt
    }

    /// Add a single example to the prompt
    fn add_example(&self, prompt: &mut String, idx: usize, template_match: &TemplateMatch) {
        let template = &template_match.template;

        writeln!(prompt, "### Example {}", idx).unwrap();
        writeln!(prompt, "**Question:** {}", template.example_question).unwrap();
        writeln!(prompt, "**SQL:** {}", template.query_pattern).unwrap();

        // Add notes about the example
        if let Some(ref description) = template.description {
            if !description.is_empty() {
                writeln!(prompt, "**Note:** {}", description).unwrap();
            }
        }

        writeln!(prompt, "**Tables:** {}", template.tables_used.join(", ")).unwrap();
        writeln!(prompt, "**Pattern:** {}\n", template.pattern_type).unwrap();
    }

    /// Add schema information to the prompt
    fn add_schema_info(&self, prompt: &mut String, tables: &[(String, Vec<String>)]) {
        writeln!(prompt, "## Available Tables\n").unwrap();

        for (table_name, columns) in tables {
            writeln!(prompt, "**{}**", table_name).unwrap();
            writeln!(prompt, "Columns: {}", columns.join(", ")).unwrap();
        }

        writeln!(prompt).unwrap();
    }

    /// Build a prompt for query explanation (for LLM response generation)
    pub fn build_explanation_prompt(
        &self,
        user_query: &str,
        query_plan: &QueryPlan,
        sql_query: &str,
        results_count: usize,
    ) -> String {
        format!(
            "You are a database assistant. Explain the SQL query results in natural language.\n\n\
            **User Question:** {}\n\
            **Query Plan:** {:?}\n\
            **SQL Query:** {}\n\
            **Results Found:** {}\n\n\
            Provide a clear, concise explanation of what was found.",
            user_query, query_plan, sql_query, results_count
        )
    }
}

impl Default for FewShotPromptBuilder {
    fn default() -> Self {
        Self::new(3, true) // Default: 3 examples with schema info
    }
}
