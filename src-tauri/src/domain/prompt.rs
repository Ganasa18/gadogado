use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Prompt {
    pub id: Option<i64>,
    #[validate(length(min = 1, max = 4096))]
    pub content: String,
    pub source_lang: String,
    pub target_lang: String,
    pub result: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Prompt {
    pub fn new(content: String, source_lang: String, target_lang: String) -> Self {
        Self {
            id: None,
            content,
            source_lang,
            target_lang,
            result: None,
            created_at: Some(chrono::Utc::now()),
        }
    }
}
