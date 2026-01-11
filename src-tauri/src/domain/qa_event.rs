use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaEventInput {
    pub event_type: String,
    pub selector: Option<String>,
    pub element_text: Option<String>,
    pub value: Option<String>,
    pub url: Option<String>,
    pub meta_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QaEvent {
    pub id: String,
    pub session_id: String,
    pub seq: i64,
    pub ts: i64,
    pub event_type: String,
    pub selector: Option<String>,
    pub element_text: Option<String>,
    pub value: Option<String>,
    pub url: Option<String>,
    pub screenshot_id: Option<String>,
    pub meta_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QaEventPage {
    pub events: Vec<QaEvent>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
