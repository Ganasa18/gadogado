use std::sync::Arc;

use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::Method;
use tauri::State;

use crate::domain::error::{AppError, Result};
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::AppState;

use super::logging::{emit_status_log, QaLogContext};
use super::types::{QaApiKeyValue, QaApiRequest, QaApiResponse};

#[tauri::command]
pub async fn qa_execute_api_request(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    request: QaApiRequest,
) -> Result<QaApiResponse> {
    let source = request
        .source
        .as_deref()
        .map(|value| format!("({}) ", value))
        .unwrap_or_default();

    let request_body_note = if let Some(body_json) = request.body_json.as_ref() {
        format!(" body_len={}", body_json.len())
    } else if !request.form_data.is_empty() {
        " body_len=form-data".to_string()
    } else {
        String::new()
    };
    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "[Request] {source}{} {}{}",
            request.method, request.url, request_body_note
        ),
    );

    let method =
        Method::from_bytes(request.method.trim().to_uppercase().as_bytes()).map_err(|_| {
            AppError::ValidationError("Invalid HTTP method for API request.".to_string())
        })?;
    let mut url = url::Url::parse(request.url.trim())
        .map_err(|_| AppError::ValidationError("Invalid URL for API request.".to_string()))?;

    if !request.query_params.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for param in request.query_params.into_iter().filter(|item| item.enabled) {
            if param.key.trim().is_empty() {
                continue;
            }
            pairs.append_pair(param.key.trim(), param.value.trim());
        }
    }

    let mut header_map = HeaderMap::new();
    for header in request.headers.into_iter().filter(|item| item.enabled) {
        if header.key.trim().is_empty() {
            continue;
        }
        let name = HeaderName::from_bytes(header.key.trim().as_bytes()).map_err(|_| {
            AppError::ValidationError("Invalid header name for API request.".to_string())
        })?;
        let value = HeaderValue::from_str(header.value.trim()).map_err(|_| {
            AppError::ValidationError("Invalid header value for API request.".to_string())
        })?;
        header_map.insert(name, value);
    }

    let client = reqwest::Client::new();
    let mut builder = client
        .request(method, url.clone())
        .headers(header_map.clone());

    if let Some(body_type) = request.body_type.as_deref() {
        if body_type == "json" {
            if let Some(body_json) = request.body_json.as_ref() {
                if !body_json.trim().is_empty() {
                    if !header_map.contains_key(reqwest::header::CONTENT_TYPE) {
                        builder = builder.header(reqwest::header::CONTENT_TYPE, "application/json");
                    }
                    builder = builder.body(body_json.trim().to_string());
                }
            }
        } else if body_type == "form" {
            let mut form = Form::new();
            for field in request.form_data.into_iter().filter(|item| item.enabled) {
                if field.key.trim().is_empty() {
                    continue;
                }
                if let Some(file_base64) = field.file_base64.as_ref() {
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(file_base64)
                        .map_err(|_| {
                            AppError::ValidationError(
                                "Failed to decode form-data file payload.".to_string(),
                            )
                        })?;
                    let mut part = Part::bytes(decoded).file_name(
                        field
                            .file_name
                            .clone()
                            .unwrap_or_else(|| "upload".to_string()),
                    );
                    if let Some(content_type) = field.content_type.as_ref() {
                        part = part.mime_str(content_type).map_err(|_| {
                            AppError::ValidationError("Invalid form-data content type.".to_string())
                        })?;
                    }
                    form = form.part(field.key.trim().to_string(), part);
                } else if let Some(value) = field.value.as_ref() {
                    form = form.text(field.key.trim().to_string(), value.clone());
                }
            }
            builder = builder.multipart(form);
        }
    }

    let start = std::time::Instant::now();
    let response = match builder.send().await {
        Ok(response) => response,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                &format!("QA API request failed: {} {}", request.method, request.url),
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: None,
                    run_id: None,
                    run_type: None,
                    mode: Some("api".to_string()),
                    event_type: Some("api_request".to_string()),
                    status_code: None,
                    latency_ms: None,
                }),
            );
            return Err(AppError::Internal(format!("API request failed: {}", err)));
        }
    };

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let mut headers = Vec::new();
    for (key, value) in response.headers().iter() {
        if let Ok(value_str) = value.to_str() {
            headers.push(QaApiKeyValue {
                key: key.to_string(),
                value: value_str.to_string(),
                enabled: true,
            });
        }
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(err) => {
            emit_status_log(
                &app,
                &state.logs,
                "ERROR",
                "QA",
                "QA API response read failed",
                "failed",
                Some(&err.to_string()),
                Some(QaLogContext {
                    session_id: None,
                    run_id: None,
                    run_type: None,
                    mode: Some("api".to_string()),
                    event_type: Some("api_response".to_string()),
                    status_code: Some(status as i64),
                    latency_ms: Some(start.elapsed().as_millis() as i64),
                }),
            );
            return Err(AppError::Internal(
                "Failed to read API response body.".to_string(),
            ));
        }
    };

    add_log(
        &state.logs,
        "INFO",
        "QA",
        &format!(
            "[Response] {source}status={} body_len={}",
            status,
            body.len()
        ),
    );
    emit_status_log(
        &app,
        &state.logs,
        "INFO",
        "QA",
        "QA API response received",
        "success",
        None,
        Some(QaLogContext {
            session_id: None,
            run_id: None,
            run_type: None,
            mode: Some("api".to_string()),
            event_type: Some("api_response".to_string()),
            status_code: Some(status as i64),
            latency_ms: Some(start.elapsed().as_millis() as i64),
        }),
    );

    Ok(QaApiResponse {
        status,
        duration_ms: start.elapsed().as_millis() as i64,
        headers,
        body,
        content_type,
    })
}
