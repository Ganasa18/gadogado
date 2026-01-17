use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    Extraction,
    Retrieval,
    Chat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_type: AnalyticsEventType,
    pub timestamp_ms: u64,
    pub success: bool,
    pub duration_ms: u64,
    pub metadata: AnalyticsMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsMetadata {
    pub doc_type: Option<String>,
    pub collection_id: Option<i64>,
    pub query_hash: Option<String>,
    pub query_length: Option<usize>,
    pub sources: Option<usize>,
    pub confidence: Option<f32>,
    pub answer_length: Option<usize>,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_events: usize,
    pub extraction_count: usize,
    pub retrieval_count: usize,
    pub chat_count: usize,
    pub avg_extraction_ms: f32,
    pub avg_retrieval_ms: f32,
    pub avg_chat_ms: f32,
    pub success_rate: f32,
}

pub struct AnalyticsLogger {
    events: Vec<AnalyticsEvent>,
    max_entries: usize,
}

impl AnalyticsLogger {
    pub fn new(max_entries: usize) -> Self {
        Self {
            events: Vec::new(),
            max_entries,
        }
    }

    pub fn log_extraction(&mut self, doc_type: &str, success: bool, duration_ms: u64) {
        let metadata = AnalyticsMetadata {
            doc_type: Some(doc_type.to_string()),
            ..Default::default()
        };
        self.push_event(AnalyticsEvent {
            event_type: AnalyticsEventType::Extraction,
            timestamp_ms: now_ms(),
            success,
            duration_ms,
            metadata,
        });
    }

    pub fn log_retrieval(
        &mut self,
        query: &str,
        collection_id: i64,
        sources: usize,
        confidence: Option<f32>,
        duration_ms: u64,
    ) {
        let metadata = AnalyticsMetadata {
            collection_id: Some(collection_id),
            query_hash: Some(hash_query(query)),
            query_length: Some(query.len()),
            sources: Some(sources),
            confidence,
            ..Default::default()
        };
        self.push_event(AnalyticsEvent {
            event_type: AnalyticsEventType::Retrieval,
            timestamp_ms: now_ms(),
            success: true,
            duration_ms,
            metadata,
        });
    }

    pub fn log_chat(
        &mut self,
        query: &str,
        collection_id: Option<i64>,
        answer_length: usize,
        feedback: Option<String>,
        duration_ms: u64,
    ) {
        let metadata = AnalyticsMetadata {
            collection_id,
            query_hash: Some(hash_query(query)),
            query_length: Some(query.len()),
            answer_length: Some(answer_length),
            feedback,
            ..Default::default()
        };
        self.push_event(AnalyticsEvent {
            event_type: AnalyticsEventType::Chat,
            timestamp_ms: now_ms(),
            success: true,
            duration_ms,
            metadata,
        });
    }

    pub fn recent_events(&self, limit: usize) -> Vec<AnalyticsEvent> {
        self.events.iter().rev().take(limit).cloned().collect()
    }

    pub fn recent_events_by_collection(
        &self,
        limit: usize,
        collection_id: Option<i64>,
    ) -> Vec<AnalyticsEvent> {
        self.events
            .iter()
            .filter(|e| collection_id.map_or(true, |id| e.metadata.collection_id == Some(id)))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn summary(&self, collection_id: Option<i64>) -> AnalyticsSummary {
        let filtered_events: Vec<&AnalyticsEvent> = if let Some(cid) = collection_id {
            self.events
                .iter()
                .filter(|e| e.metadata.collection_id == Some(cid))
                .collect()
        } else {
            self.events.iter().collect()
        };

        let mut extraction_count = 0usize;
        let mut retrieval_count = 0usize;
        let mut chat_count = 0usize;
        let mut extraction_ms = 0u64;
        let mut retrieval_ms = 0u64;
        let mut chat_ms = 0u64;
        let mut success_count = 0usize;

        for event in &filtered_events {
            if event.success {
                success_count += 1;
            }
            match event.event_type {
                AnalyticsEventType::Extraction => {
                    extraction_count += 1;
                    extraction_ms += event.duration_ms;
                }
                AnalyticsEventType::Retrieval => {
                    retrieval_count += 1;
                    retrieval_ms += event.duration_ms;
                }
                AnalyticsEventType::Chat => {
                    chat_count += 1;
                    chat_ms += event.duration_ms;
                }
            }
        }

        let avg_extraction_ms = avg_ms(extraction_ms, extraction_count);
        let avg_retrieval_ms = avg_ms(retrieval_ms, retrieval_count);
        let avg_chat_ms = avg_ms(chat_ms, chat_count);
        let total_events = filtered_events.len();
        let success_rate = if total_events == 0 {
            0.0
        } else {
            success_count as f32 / total_events as f32
        };

        AnalyticsSummary {
            total_events,
            extraction_count,
            retrieval_count,
            chat_count,
            avg_extraction_ms,
            avg_retrieval_ms,
            avg_chat_ms,
            success_rate,
        }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    fn push_event(&mut self, event: AnalyticsEvent) {
        self.events.push(event);
        if self.events.len() > self.max_entries {
            let overflow = self.events.len() - self.max_entries;
            self.events.drain(0..overflow);
        }
    }
}

pub struct SharedAnalyticsLogger {
    inner: Arc<Mutex<AnalyticsLogger>>,
}

impl SharedAnalyticsLogger {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AnalyticsLogger::new(max_entries))),
        }
    }

    pub fn recent_events_by_collection(
        &self,
        limit: usize,
        collection_id: Option<i64>,
    ) -> Vec<AnalyticsEvent> {
        self.inner
            .lock()
            .unwrap()
            .recent_events_by_collection(limit, collection_id)
    }

    pub fn summary_for_collection(&self, collection_id: Option<i64>) -> AnalyticsSummary {
        self.inner.lock().unwrap().summary(collection_id)
    }

    pub fn log_extraction(&self, doc_type: &str, success: bool, duration_ms: u64) {
        self.inner
            .lock()
            .unwrap()
            .log_extraction(doc_type, success, duration_ms);
    }

    pub fn log_retrieval(
        &self,
        query: &str,
        collection_id: i64,
        sources: usize,
        confidence: Option<f32>,
        duration_ms: u64,
    ) {
        self.inner.lock().unwrap().log_retrieval(
            query,
            collection_id,
            sources,
            confidence,
            duration_ms,
        );
    }

    pub fn log_chat(
        &self,
        query: &str,
        collection_id: Option<i64>,
        answer_length: usize,
        feedback: Option<String>,
        duration_ms: u64,
    ) {
        self.inner.lock().unwrap().log_chat(
            query,
            collection_id,
            answer_length,
            feedback,
            duration_ms,
        );
    }

    pub fn recent_events(&self, limit: usize) -> Vec<AnalyticsEvent> {
        self.inner.lock().unwrap().recent_events(limit)
    }

    pub fn summary(&self) -> AnalyticsSummary {
        self.inner.lock().unwrap().summary(None)
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

impl Clone for SharedAnalyticsLogger {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn hash_query(query: &str) -> String {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn avg_ms(total: u64, count: usize) -> f32 {
    if count == 0 {
        0.0
    } else {
        total as f32 / count as f32
    }
}
