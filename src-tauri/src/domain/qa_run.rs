use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaSessionRun {
    pub id: String,
    pub session_id: String,
    pub run_type: String,
    pub mode: String,
    pub status: String,
    pub triggered_by: String,
    pub source_run_id: Option<String>,
    pub checkpoint_id: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub meta_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaRunStreamEvent {
    pub id: String,
    pub run_id: String,
    pub seq: i64,
    pub ts: i64,
    pub channel: String,
    pub level: String,
    pub message: String,
    pub payload_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaRunStreamInput {
    pub channel: String,
    pub level: String,
    pub message: String,
    pub payload_json: Option<String>,
}
