use actix_web::dev::ServerHandle;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, timeout};
use std::time::Duration;

use crate::domain::error::{AppError, Result};
use crate::interfaces::http::{add_log, LogEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockServerConfig {
    pub port: u16,
    #[serde(default)]
    pub routes: Vec<MockRoute>,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            port: 4010,
            routes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockRoute {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub matchers: MockRouteMatchers,
    #[serde(default)]
    pub response: MockResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockRouteMatchers {
    #[serde(default)]
    pub query_params: Vec<MockKeyValue>,
    #[serde(default)]
    pub headers: Vec<MockKeyValue>,
    pub body: Option<MockBodyMatch>,
}

impl Default for MockRouteMatchers {
    fn default() -> Self {
        Self {
            query_params: Vec::new(),
            headers: Vec::new(),
            body: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockBodyMatch {
    pub mode: MatchMode,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    Exact,
    Contains,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockKeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<MockKeyValue>,
    pub body: String,
    pub delay_ms: Option<u64>,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: "{}".to_string(),
            delay_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockServerStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
    pub route_count: usize,
}

#[derive(Clone)]
pub struct MockServerState {
    pub config: Arc<Mutex<MockServerConfig>>,
    pub server: Arc<Mutex<Option<ServerHandle>>>,
    pub config_path: PathBuf,
    pub logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl MockServerState {
    pub fn new(config_path: PathBuf, logs: Arc<Mutex<Vec<LogEntry>>>) -> Self {
        let config = match load_config_from_path(&config_path) {
            Ok(config) => config,
            Err(err) => {
                add_log(
                    &logs,
                    "ERROR",
                    "MockServer",
                    &format!("Failed to load mock server config: {}", err),
                );
                MockServerConfig::default()
            }
        };
        Self {
            config: Arc::new(Mutex::new(config)),
            server: Arc::new(Mutex::new(None)),
            config_path,
            logs,
        }
    }
}

pub fn load_config_from_path(path: &PathBuf) -> Result<MockServerConfig> {
    if !path.exists() {
        return Ok(MockServerConfig::default());
    }
    let content = fs::read_to_string(path)
        .map_err(|err| AppError::Internal(format!("Failed to read mock server config: {}", err)))?;
    serde_json::from_str(&content)
        .map_err(|err| AppError::Internal(format!("Failed to parse mock server config: {}", err)))
}

pub fn save_config(state: &MockServerState) -> Result<()> {
    let config = state.config.lock().unwrap();
    let serialized = serde_json::to_string_pretty(&*config)
        .map_err(|err| AppError::Internal(format!("Failed to serialize mock server config: {}", err)))?;
    fs::write(&state.config_path, serialized)
        .map_err(|err| AppError::Internal(format!("Failed to save mock server config: {}", err)))?;
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        &format!("Mock server config saved at {}", state.config_path.display()),
    );
    Ok(())
}

pub async fn start_mock_server(state: Arc<MockServerState>) -> Result<()> {
    let port = { state.config.lock().unwrap().port };
    let mut server_guard = state.server.lock().unwrap();
    if server_guard.is_some() {
        add_log(
            &state.logs,
            "INFO",
            "MockServer",
            "Mock server start requested but already running",
        );
        return Err(AppError::ValidationError(
            "Mock server is already running.".to_string(),
        ));
    }

    let server_state = state.clone();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server_state.clone()))
            .default_service(web::route().to(handle_mock_request))
    })
    .bind(("127.0.0.1", port))
    .map_err(|err| AppError::Internal(format!("Failed to bind mock server: {}", err)))?
    .run();

    let handle = server.handle();
    *server_guard = Some(handle);
    tokio::spawn(server);

    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        &format!("Mock server started on http://127.0.0.1:{}", port),
    );

    Ok(())
}

pub async fn stop_mock_server(state: Arc<MockServerState>) -> Result<()> {
    let handle = { state.server.lock().unwrap().take() };
    if let Some(handle) = handle {
        let graceful = timeout(Duration::from_secs(2), handle.stop(true)).await;
        if graceful.is_err() {
            handle.stop(false).await;
            add_log(
                &state.logs,
                "WARN",
                "MockServer",
                "Mock server forced stop after timeout",
            );
        } else {
            add_log(
                &state.logs,
                "INFO",
                "MockServer",
                "Mock server stopped",
            );
        }
    } else {
        add_log(
            &state.logs,
            "INFO",
            "MockServer",
            "Mock server stop requested but already stopped",
        );
    }
    Ok(())
}

pub fn build_status(state: &MockServerState) -> MockServerStatus {
    let config = state.config.lock().unwrap();
    let running = state.server.lock().unwrap().is_some();
    MockServerStatus {
        running,
        port: config.port,
        url: format!("http://127.0.0.1:{}", config.port),
        route_count: config.routes.len(),
    }
}

async fn handle_mock_request(
    req: HttpRequest,
    body: web::Bytes,
    data: web::Data<Arc<MockServerState>>,
) -> HttpResponse {
    let method = req.method().as_str().to_uppercase();
    let path = req.path().to_string();
    let body_text = String::from_utf8_lossy(&body).to_string();
    let query_map = parse_query(req.query_string());
    let headers_map = parse_headers(&req);

    let config = data.config.lock().unwrap().clone();
    let mut best_match: Option<(MockRoute, i32)> = None;

    let enabled_routes: Vec<MockRoute> = config
        .routes
        .iter()
        .filter(|route| route.enabled)
        .cloned()
        .collect();

    for route in enabled_routes.iter() {
        if !method_matches(route, &method) || !path_matches(route, &path) {
            continue;
        }
        if let Some(score) = calculate_match_score(route, &query_map, &headers_map, &body_text) {
            if let Some((_, best_score)) = best_match.as_ref() {
                if score > *best_score {
                    best_match = Some((route.clone(), score));
                }
            } else {
                best_match = Some((route.clone(), score));
            }
        }
    }

    if let Some((route, _score)) = best_match {
        if let Some(delay_ms) = route.response.delay_ms {
            if delay_ms > 0 {
                sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        add_log(
            &data.logs,
            "INFO",
            "MockServer",
            &format!(
                "Mock response served (method={} path={} route={})",
                method, path, route.name
            ),
        );

        let mut response = HttpResponse::build(
            actix_web::http::StatusCode::from_u16(route.response.status)
                .unwrap_or(actix_web::http::StatusCode::OK),
        );
        let has_content_type = route
            .response
            .headers
            .iter()
            .filter(|header| header.enabled)
            .any(|header| header.key.eq_ignore_ascii_case("content-type"));

        for header in route.response.headers.iter().filter(|item| item.enabled) {
            if !header.key.trim().is_empty() {
                response.append_header((header.key.clone(), header.value.clone()));
            }
        }

        if !has_content_type {
            if route.response.body.trim_start().starts_with('{')
                || route.response.body.trim_start().starts_with('[')
            {
                response.append_header(("Content-Type", "application/json"));
            } else {
                response.append_header(("Content-Type", "text/plain"));
            }
        }

        return response.body(route.response.body);
    }

    add_log(
        &data.logs,
        "INFO",
        "MockServer",
        &format!(
            "Mock response not found (method={} path={} enabled_routes={})",
            method,
            path,
            enabled_routes.len()
        ),
    );

    HttpResponse::NotFound().json(serde_json::json!({
        "error": "No mock route matched.",
        "method": method,
        "path": path
    }))
}

fn method_matches(route: &MockRoute, method: &str) -> bool {
    route.method.trim().eq_ignore_ascii_case(method)
}

fn path_matches(route: &MockRoute, path: &str) -> bool {
    let route_path = route.path.trim();
    if route_path.is_empty() {
        return false;
    }
    let normalized_route = normalize_path(route_path);
    let normalized_path = normalize_path(path);
    normalized_route == normalized_path
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed == "/" {
        return "/".to_string();
    }
    trimmed.trim_end_matches('/').to_string()
}

fn calculate_match_score(
    route: &MockRoute,
    query_map: &HashMap<String, String>,
    header_map: &HashMap<String, String>,
    body_text: &str,
) -> Option<i32> {
    let mut score = 0;

    let query_enabled = route.matchers.query_params.iter().any(|rule| rule.enabled);
    if query_enabled {
        if !match_key_values(&route.matchers.query_params, query_map) {
            return None;
        }
        score += 1;
    }

    let header_enabled = route.matchers.headers.iter().any(|rule| rule.enabled);
    if header_enabled {
        if !match_key_values(&route.matchers.headers, header_map) {
            return None;
        }
        score += 1;
    }

    if let Some(body_match) = &route.matchers.body {
        if !match_body(body_match, body_text) {
            return None;
        }
        score += 1;
    }

    Some(score)
}

fn match_key_values(rules: &[MockKeyValue], values: &HashMap<String, String>) -> bool {
    let enabled_rules: Vec<&MockKeyValue> = rules.iter().filter(|rule| rule.enabled).collect();
    if enabled_rules.is_empty() {
        return true;
    }
    enabled_rules.into_iter().all(|rule| {
        let key = rule.key.trim().to_lowercase();
        if key.is_empty() {
            return true;
        }
        values
            .get(&key)
            .map(|value| {
                if rule.value.trim().is_empty() {
                    true
                } else {
                    value == rule.value.trim()
                }
            })
            .unwrap_or(false)
    })
}

fn match_body(rule: &MockBodyMatch, body_text: &str) -> bool {
    let rule_value = rule.value.trim();
    if rule_value.is_empty() {
        return true;
    }
    match rule.mode {
        MatchMode::Exact => body_text.trim() == rule_value,
        MatchMode::Contains => body_text.contains(rule_value),
        MatchMode::Regex => Regex::new(rule_value).map(|re| re.is_match(body_text)).unwrap_or(false),
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .map(|(key, value)| (key.to_lowercase(), value))
        .collect()
}

fn parse_headers(req: &HttpRequest) -> HashMap<String, String> {
    req.headers()
        .iter()
        .filter_map(|(key, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (key.to_string().to_lowercase(), value.to_string()))
        })
        .collect()
}
