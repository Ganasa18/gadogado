use crate::domain::error::{AppError, Result};
use crate::domain::qa_api_call::QaApiCall;
use crate::infrastructure::db::qa_api_calls::QaApiCallRepository;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use uuid::Uuid;

const BODY_PREVIEW_LIMIT: usize = 4000;

pub struct QaApiCallUseCase {
    repository: Arc<QaApiCallRepository>,
}

impl QaApiCallUseCase {
    pub fn new(repository: Arc<QaApiCallRepository>) -> Self {
        Self { repository }
    }

    pub async fn record_api_call(
        &self,
        session_id: &str,
        run_id: &str,
        method: &str,
        url: &str,
        request_headers_json: Option<String>,
        request_body: Option<String>,
        response_status: Option<i64>,
        response_headers_json: Option<String>,
        response_body: Option<String>,
        timing_ms: Option<i64>,
    ) -> Result<QaApiCall> {
        let session_id = session_id.trim();
        let run_id = run_id.trim();
        let method = method.trim();
        let url = url.trim();

        if session_id.is_empty() || run_id.is_empty() || method.is_empty() || url.is_empty() {
            return Err(AppError::ValidationError(
                "API call requires session, run, method, and url.".to_string(),
            ));
        }

        let request_body_hash = request_body.as_ref().map(|body| hash_body(body));
        let response_body_hash = response_body.as_ref().map(|body| hash_body(body));
        let request_body_json = request_body
            .as_ref()
            .map(|body| truncate_body(body));

        let call = QaApiCall {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            run_id: run_id.to_string(),
            method: method.to_string(),
            url: url.to_string(),
            request_headers_json,
            request_body_json,
            request_body_hash,
            response_status,
            response_headers_json,
            response_body_hash,
            timing_ms,
            created_at: chrono::Utc::now().timestamp_millis(),
        };

        self.repository.insert_call(&call).await?;
        Ok(call)
    }
}

fn hash_body(body: &str) -> String {
    let mut hasher = DefaultHasher::new();
    body.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn truncate_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.len() <= BODY_PREVIEW_LIMIT {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..BODY_PREVIEW_LIMIT])
    }
}
