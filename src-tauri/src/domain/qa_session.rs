use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QaSession {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub session_type: String,
    pub is_positive_case: bool,
    pub target_url: Option<String>,
    pub api_base_url: Option<String>,
    pub auth_profile_json: Option<String>,
    pub source_session_id: Option<String>,
    pub app_version: Option<String>,
    pub os: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub notes: Option<String>,
}
