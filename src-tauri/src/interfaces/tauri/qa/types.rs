use serde::{Deserialize, Serialize};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaScreenshotResult {
    pub path: String,
    pub event_id: String,
    pub artifact_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiKeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiFormField {
    pub key: String,
    pub value: Option<String>,
    pub file_name: Option<String>,
    pub file_base64: Option<String>,
    pub content_type: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<QaApiKeyValue>,
    pub query_params: Vec<QaApiKeyValue>,
    pub body_type: Option<String>,
    pub body_json: Option<String>,
    pub form_data: Vec<QaApiFormField>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaBrowserReplayEvent {
    pub event_type: String,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub url: Option<String>,
    pub ts: i64,
    pub seq: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaBrowserReplayPayload {
    pub target_url: String,
    pub events: Vec<QaBrowserReplayEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaApiResponse {
    pub status: u16,
    pub duration_ms: i64,
    pub headers: Vec<QaApiKeyValue>,
    pub body: String,
    pub content_type: Option<String>,
}
