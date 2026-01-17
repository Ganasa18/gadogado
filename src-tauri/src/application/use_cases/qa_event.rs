use crate::domain::error::{AppError, Result};
use crate::domain::qa_event::{QaEvent, QaEventInput, QaEventPage, QaEventSummary};
use crate::infrastructure::db::qa_events::QaEventRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct QaEventUseCase {
    repository: Arc<QaEventRepository>,
}

impl QaEventUseCase {
    pub fn new(repository: Arc<QaEventRepository>) -> Self {
        Self { repository }
    }

    pub async fn record_event(&self, session_id: &str, input: QaEventInput) -> Result<QaEvent> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }

        let event_type = input.event_type.trim().to_lowercase();
        if event_type.is_empty() {
            return Err(AppError::ValidationError(
                "Event type is required.".to_string(),
            ));
        }
        let is_supported = matches!(
            event_type.as_str(),
            "click"
                | "input"
                | "submit"
                | "navigation"
                | "change"
                | "dblclick"
                | "contextmenu"
                | "keydown"
                | "keyup"
                | "focus"
                | "blur"
                | "scroll"
                | "resize"
        ) || event_type.starts_with("curl_")
            || event_type.starts_with("api_");

        if !is_supported {
            return Err(AppError::ValidationError(format!(
                "Unsupported event type: {}",
                event_type
            )));
        }

        let event = QaEvent {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            run_id: normalize_optional(input.run_id),
            checkpoint_id: normalize_optional(input.checkpoint_id),
            seq: 0,
            ts: chrono::Utc::now().timestamp_millis(),
            event_type,
            origin: normalize_optional(input.origin),
            recording_mode: normalize_optional(input.recording_mode),
            selector: normalize_optional(input.selector),
            element_text: normalize_optional(input.element_text),
            value: normalize_value(input.value),
            url: normalize_optional(input.url),
            screenshot_id: None,
            screenshot_path: None,
            meta_json: normalize_optional(input.meta_json),
        };

        self.repository.insert_event(event).await
    }

    pub async fn list_events(&self, session_id: &str) -> Result<Vec<QaEvent>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.repository.list_events(session_id).await
    }

    pub async fn list_screenshots(&self, session_id: &str) -> Result<Vec<QaEvent>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.repository.list_screenshots(session_id).await
    }

    pub async fn list_events_page(
        &self,
        session_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<QaEventPage> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        if page <= 0 || page_size <= 0 {
            return Err(AppError::ValidationError(
                "Page and page size must be positive.".to_string(),
            ));
        }

        let total = self.repository.count_events(session_id).await?;
        let offset = (page - 1) * page_size;
        let events = if total == 0 {
            Vec::new()
        } else {
            self.repository
                .list_events_page(session_id, page_size, offset)
                .await?
        };

        Ok(QaEventPage {
            events,
            total,
            page,
            page_size,
        })
    }

    pub async fn delete_events(&self, session_id: &str, event_ids: Vec<String>) -> Result<u64> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        let cleaned: Vec<String> = event_ids
            .into_iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect();

        if cleaned.is_empty() {
            return Ok(0);
        }

        self.repository.delete_events(session_id, &cleaned).await
    }

    pub async fn latest_event_summary(&self, session_id: &str) -> Result<Option<QaEventSummary>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.repository.latest_event_summary(session_id).await
    }

    pub async fn attach_screenshot(
        &self,
        session_id: &str,
        event_id: Option<String>,
        artifact_id: &str,
        path: &str,
        mime: Option<&str>,
        width: Option<i64>,
        height: Option<i64>,
        created_at: i64,
    ) -> Result<String> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }

        let resolved_event_id = match event_id {
            Some(event_id) if !event_id.trim().is_empty() => event_id,
            _ => self
                .repository
                .latest_event_id(session_id)
                .await?
                .ok_or_else(|| {
                    AppError::ValidationError("No QA event available for screenshot.".to_string())
                })?,
        };

        self.repository
            .attach_screenshot(
                session_id,
                &resolved_event_id,
                artifact_id,
                path,
                mime,
                width,
                height,
                created_at,
            )
            .await?;

        Ok(resolved_event_id)
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|val| {
        let trimmed = val.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_value(value: Option<String>) -> Option<String> {
    value.and_then(|val| {
        if val.trim().is_empty() {
            None
        } else {
            Some(val)
        }
    })
}
