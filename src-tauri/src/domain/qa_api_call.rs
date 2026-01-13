use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaApiCall {
    pub id: String,
    pub session_id: String,
    pub run_id: String,
    pub method: String,
    pub url: String,
    pub request_headers_json: Option<String>,
    pub request_body_json: Option<String>,
    pub request_body_hash: Option<String>,
    pub response_status: Option<i64>,
    pub response_headers_json: Option<String>,
    pub response_body_hash: Option<String>,
    pub timing_ms: Option<i64>,
    pub created_at: i64,
}
