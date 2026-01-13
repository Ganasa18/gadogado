use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaCheckpoint {
    pub id: String,
    pub session_id: String,
    pub seq: i64,
    pub title: Option<String>,
    pub start_event_seq: i64,
    pub end_event_seq: i64,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaCheckpointSummary {
    pub id: String,
    pub checkpoint_id: String,
    pub summary_text: String,
    pub entities_json: Option<String>,
    pub risks_json: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaTestCase {
    pub id: String,
    pub session_id: String,
    pub checkpoint_id: Option<String>,
    #[serde(rename = "type")]
    pub case_type: String,
    pub title: String,
    pub steps_json: String,
    pub expected: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub dedup_hash: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaLlmRun {
    pub id: String,
    pub scope: String,
    pub scope_id: String,
    pub model: String,
    pub prompt_version: Option<String>,
    pub input_digest: Option<String>,
    pub input_summary: Option<String>,
    pub output_json: String,
    pub created_at: i64,
}
