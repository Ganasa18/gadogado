use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::typegen::TypeGenMode;
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::response::clean_llm_response;
use serde_json::Value;
use std::sync::Arc;

mod language;
mod naming;
mod renderer;
mod schema;

use language::TargetLanguage;
use naming::sanitize_type_name;
use renderer::TypeRenderer;
use schema::infer_schema;

const MAX_JSON_CHARS: usize = 16_384;

pub struct TypeGenUseCase {
    llm_client: Arc<dyn LLMClient + Send + Sync>,
}

impl TypeGenUseCase {
    pub fn new(llm_client: Arc<dyn LLMClient + Send + Sync>) -> Self {
        Self { llm_client }
    }

    pub async fn execute(
        &self,
        config: &LLMConfig,
        json_input: String,
        language: String,
        root_name: String,
        mode: TypeGenMode,
    ) -> Result<String> {
        let sanitized = strip_control_chars(&json_input);
        if sanitized.len() > MAX_JSON_CHARS {
            return Err(AppError::ValidationError(format!(
                "JSON exceeds {} characters.",
                MAX_JSON_CHARS
            )));
        }

        let parsed: Value = serde_json::from_str(&sanitized)
            .map_err(|e| AppError::ValidationError(format!("Invalid JSON input: {}", e)))?;
        let schema = infer_schema(&parsed);

        let pretty_json = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| sanitized);
        let root = if root_name.trim().is_empty() {
            "Root".to_string()
        } else {
            root_name.trim().to_string()
        };
        let language = language.trim().to_string();

        match mode {
            TypeGenMode::Offline => generate_offline(&schema, &root, &language),
            TypeGenMode::Llm => generate_llm(self, config, &pretty_json, &language, &root).await,
            TypeGenMode::Auto => {
                let llm_result = generate_llm(self, config, &pretty_json, &language, &root).await;
                match llm_result {
                    Ok(result) => Ok(result),
                    Err(llm_err) => match generate_offline(&schema, &root, &language) {
                        Ok(result) => Ok(result),
                        Err(_) => Err(llm_err),
                    },
                }
            }
        }
    }
}

async fn generate_llm(
    use_case: &TypeGenUseCase,
    config: &LLMConfig,
    pretty_json: &str,
    language: &str,
    root_name: &str,
) -> Result<String> {
    let system_prompt = build_system_prompt(language, root_name);
    let user_prompt = format!("JSON:\n{}", pretty_json);

    let raw_result = use_case
        .llm_client
        .generate(config, &system_prompt, &user_prompt)
        .await?;

    Ok(clean_llm_response(&raw_result))
}

fn generate_offline(schema: &schema::Schema, root_name: &str, language: &str) -> Result<String> {
    let lang = TargetLanguage::parse(language).ok_or_else(|| {
        AppError::ValidationError(format!(
            "Unsupported language for offline mode: {}",
            language
        ))
    })?;
    let root_name = sanitize_type_name(root_name, lang);
    let renderer = TypeRenderer::new(lang, &root_name);
    Ok(renderer.render(schema, &root_name))
}

fn strip_control_chars(input: &str) -> String {
    input.chars().filter(|ch| (*ch as u32) >= 0x20).collect()
}

fn build_system_prompt(language: &str, root_name: &str) -> String {
    format!(
        "You are a code generator. Generate type definitions from a JSON response.\n\
Target language: {language}\n\
Root type name: {root_name}\n\
Requirements:\n\
- Output ONLY code. No markdown, no explanations.\n\
- Define all nested object and array types.\n\
- Use idiomatic naming for types in the target language.\n\
- Preserve JSON field names. If a field name is not a valid identifier, use a safe identifier and add a mapping using the target language's conventions.\n\
- Use nullable/optional types when JSON values can be null.\n\
- Treat all JSON strings as plain string types; do not infer date/time types.\n\
- Do not include parsing, constructors, or helper functions.\n\
Language rules:\n\
- Go: use struct types and json tags.\n\
- Rust: use struct types with serde derive and serde rename when needed.\n\
- TypeScript: use interfaces and quoted property names when needed.\n\
- Dart: use class types with typed fields only; if needed, add a comment like: // json: \"original\".\n\
- Flutter: use Dart class types with typed fields only; if needed, add a comment like: // json: \"original\".\n\
- Java: use class types with public fields; use List<T> for arrays and boxed types when needed.\n\
- PHP: use class types with typed public properties; if needed, add a comment like: // json: \"original\".\n\
"
    )
}
