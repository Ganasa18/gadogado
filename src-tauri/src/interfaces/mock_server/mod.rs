use actix_web::dev::ServerHandle;
use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{sleep, timeout};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStrategy {
    #[default]
    Single,
    Multi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MultiResponseMatchMode {
    #[default]
    Exact,
    #[serde(rename = "keymatch", alias = "key_match")]
    KeyMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadResponseMapping {
    pub id: String,
    pub name: String,
    pub payload: String,
    #[serde(default)]
    pub response: MockResponse,
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
    pub response_strategy: ResponseStrategy,
    #[serde(default)]
    pub multi_response_match_mode: MultiResponseMatchMode,
    #[serde(default)]
    pub response: MockResponse,
    #[serde(default)]
    pub multi_responses: Vec<PayloadResponseMapping>,
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
    #[serde(default)]
    pub body_type: BodyType,
    #[serde(default)]
    pub form_data: Vec<FormDataItem>,
    #[serde(default)]
    pub form_urlencode: Vec<MockKeyValue>,
    #[serde(default)]
    pub validation_strategy: ValidationStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStrategy {
    Exact,
    KeyOnly,
}

impl Default for ValidationStrategy {
    fn default() -> Self {
        ValidationStrategy::Exact
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    Exact,
    Contains,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    RawJson,
    RawXml,
    FormData,
    FormUrlencode,
}

impl Default for BodyType {
    fn default() -> Self {
        BodyType::RawJson
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseBodyType {
    None,
    FormData,
    FormUrlencode,
    Raw,
}

impl Default for ResponseBodyType {
    fn default() -> Self {
        ResponseBodyType::Raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RawSubType {
    Text,
    Json,
    Xml,
    Html,
    Javascript,
}

impl Default for RawSubType {
    fn default() -> Self {
        RawSubType::Json
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormDataFieldType {
    Text,
    File,
}

impl Default for FormDataFieldType {
    fn default() -> Self {
        FormDataFieldType::Text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormDataItem {
    pub key: String,
    pub value: String,
    #[serde(rename = "type", default)]
    pub field_type: FormDataFieldType,
    #[serde(default)]
    pub enabled: bool,
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
    #[serde(default)]
    pub body_type: ResponseBodyType,
    #[serde(default)]
    pub raw_sub_type: RawSubType,
    #[serde(default)]
    pub form_data: Vec<FormDataItem>,
    #[serde(default)]
    pub form_urlencode: Vec<MockKeyValue>,
    #[serde(default)]
    pub body: String,
    pub delay_ms: Option<u64>,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: "{}".to_string(),
            body_type: ResponseBodyType::Raw,
            raw_sub_type: RawSubType::Json,
            form_data: Vec::new(),
            form_urlencode: Vec::new(),
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
    let serialized = serde_json::to_string_pretty(&*config).map_err(|err| {
        AppError::Internal(format!("Failed to serialize mock server config: {}", err))
    })?;
    fs::write(&state.config_path, serialized)
        .map_err(|err| AppError::Internal(format!("Failed to save mock server config: {}", err)))?;
    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        &format!(
            "Mock server config saved at {}",
            state.config_path.display()
        ),
    );
    Ok(())
}

pub async fn start_mock_server(state: Arc<MockServerState>) -> Result<()> {
    let port = { state.config.lock().unwrap().port };

    // Check if already running (quick lock check)
    {
        let server_guard = state.server.lock().unwrap();
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
    } // Release lock before creating server

    add_log(
        &state.logs,
        "INFO",
        "MockServer",
        &format!("Starting mock server on port {}...", port),
    );

    let server_state = state.clone();
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    // Mock server is typically consumed by browsers/tools during local dev.
                    // Allow any origin so requests from localhost:4200, etc. work out of the box.
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .app_data(web::Data::new(server_state.clone()))
            .default_service(web::route().to(handle_mock_request))
    })
    .bind(("127.0.0.1", port))
    .map_err(|err| {
        add_log(
            &state.logs,
            "ERROR",
            "MockServer",
            &format!("Failed to bind mock server on port {}: {}", port, err),
        );
        AppError::Internal(format!("Failed to bind mock server: {}", err))
    })?
    .run();

    let handle = server.handle();

    // Store the handle
    {
        let mut server_guard = state.server.lock().unwrap();
        *server_guard = Some(handle);
    }

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
    add_log(&state.logs, "INFO", "MockServer", "Stopping mock server...");
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
                "Mock server stopped gracefully",
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

    // Step 1: Find best route match by method + path + query + headers (NO body check)
    for route in enabled_routes.iter() {
        if !method_matches(route, &method) || !path_matches(route, &path) {
            continue;
        }
        if let Some(score) = calculate_route_score(route, &query_map, &headers_map) {
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
        // Step 2: Validate body based on response strategy
        // Body mismatch → 400 (route found but payload wrong)
        let response = match route.response_strategy {
            ResponseStrategy::Single => {
                // Validate body against matchers if configured
                if let Some(body_match) = &route.matchers.body {
                    if !match_body(body_match, &body_text) {
                        add_log(
                            &data.logs,
                            "WARN",
                            "MockServer",
                            &format!(
                                "Route matched but body validation failed (route={})",
                                route.name
                            ),
                        );
                        return HttpResponse::BadRequest().json(serde_json::json!({
                            "error": "Body validation failed",
                            "route": route.name,
                            "received_body": body_text
                        }));
                    }
                }
                &route.response
            }
            ResponseStrategy::Multi => {
                match find_matching_response(&route, &body_text, route.multi_response_match_mode) {
                    Some(resp) => resp,
                    None => {
                        // KeyMatch: if no mapping matches, fall back to the default route response.
                        // Exact: strict; missing mapping is a 400.
                        if route.multi_response_match_mode == MultiResponseMatchMode::KeyMatch {
                            add_log(
                                &data.logs,
                                "INFO",
                                "MockServer",
                                &format!(
                                    "Multi-response KeyMatch fallback to default response (route={})",
                                    route.name
                                ),
                            );
                            &route.response
                        } else {
                            add_log(
                                &data.logs,
                                "WARN",
                                "MockServer",
                                &format!(
                                    "Multi-response route matched but no payload match found (route={}, mode={:?})",
                                    route.name, route.multi_response_match_mode
                                ),
                            );
                            return HttpResponse::BadRequest().json(serde_json::json!({
                                "error": "No matching payload found for this route",
                                "route": route.name,
                                "match_mode": format!("{:?}", route.multi_response_match_mode),
                                "received_body": body_text
                            }));
                        }
                    }
                }
            }
        };

        if let Some(delay_ms) = response.delay_ms {
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

        let mut http_response = HttpResponse::build(
            actix_web::http::StatusCode::from_u16(response.status)
                .unwrap_or(actix_web::http::StatusCode::OK),
        );
        let has_content_type = response
            .headers
            .iter()
            .filter(|header| header.enabled)
            .any(|header| header.key.eq_ignore_ascii_case("content-type"));

        for header in response.headers.iter().filter(|item| item.enabled) {
            if !header.key.trim().is_empty() {
                http_response.append_header((header.key.clone(), header.value.clone()));
            }
        }

        if !has_content_type {
            if response.body.trim_start().starts_with('{')
                || response.body.trim_start().starts_with('[')
            {
                http_response.append_header(("Content-Type", "application/json"));
            } else {
                http_response.append_header(("Content-Type", "text/plain"));
            }
        }

        return http_response.body(response.body.clone());
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

/// Calculate route match score based on query params and headers only (no body).
/// Body validation is done separately after route selection to distinguish 400 vs 404.
fn calculate_route_score(
    route: &MockRoute,
    query_map: &HashMap<String, String>,
    header_map: &HashMap<String, String>,
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
    match rule.body_type {
        BodyType::FormUrlencode => {
            if !match_form_urlencode(&rule.form_urlencode, body_text) {
                return false;
            }
        }
        BodyType::FormData => {
            if !match_form_data(&rule.form_data, body_text) {
                return false;
            }
        }
        BodyType::RawJson => {
            if let Some(result) = match_json_body(rule, body_text) {
                return result;
            }
        }
        BodyType::RawXml => {}
    }

    if rule.validation_strategy == ValidationStrategy::KeyOnly {
        return match_body_keys(rule, body_text);
    }

    match_body_value(rule, body_text)
}

fn match_body_keys(rule: &MockBodyMatch, body_text: &str) -> bool {
    match rule.body_type {
        BodyType::FormUrlencode => {
            let parsed = parse_form_body(body_text);
            rule.form_urlencode.iter().filter(|r| r.enabled).all(|r| {
                let key = r.key.trim().to_lowercase();
                key.is_empty() || parsed.contains_key(&key)
            })
        }
        BodyType::FormData => rule.form_data.iter().filter(|r| r.enabled).all(|r| {
            let key = r.key.trim();
            if key.is_empty() {
                return true;
            }
            let name_token = format!("name=\"{}\"", key);
            body_text.contains(&name_token)
        }),
        BodyType::RawJson => {
            let rule_json = match parse_json_with_comments(&rule.value) {
                Some(j) => j,
                None => return true, // If rule is invalid JSON, just pass
            };
            let body_json = match parse_json_with_comments(body_text) {
                Some(j) => j,
                None => return false,
            };
            json_keys_match(&body_json, &rule_json)
        }
        BodyType::RawXml => {
            // Basic XML key (tag) matching could be complex,
            // for now just check if tag names exist in string
            // as a simple fallback
            true
        }
    }
}

fn json_keys_match(haystack: &JsonValue, needle: &JsonValue) -> bool {
    match (haystack, needle) {
        (JsonValue::Object(hay), JsonValue::Object(need)) => {
            need.keys().all(|k| hay.contains_key(k))
        }
        (JsonValue::Array(hay), JsonValue::Array(need)) => {
            // For arrays, we just check if they are both arrays for now
            // Or we could check if items match keys
            !hay.is_empty() || need.is_empty()
        }
        _ => true, // Fallback for primitive types
    }
}

fn match_body_value(rule: &MockBodyMatch, body_text: &str) -> bool {
    let rule_value = rule.value.trim();
    if rule_value.is_empty() {
        return true;
    }
    match rule.mode {
        MatchMode::Exact => body_text.trim() == rule_value,
        MatchMode::Contains => body_text.contains(rule_value),
        MatchMode::Regex => Regex::new(rule_value)
            .map(|re| re.is_match(body_text))
            .unwrap_or(false),
    }
}

fn match_form_urlencode(rules: &[MockKeyValue], body_text: &str) -> bool {
    let enabled_rules: Vec<&MockKeyValue> = rules.iter().filter(|rule| rule.enabled).collect();
    if enabled_rules.is_empty() {
        return true;
    }
    let parsed = parse_form_body(body_text);
    match_key_values(rules, &parsed)
}

fn parse_form_body(body_text: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(body_text.as_bytes())
        .into_owned()
        .map(|(key, value)| (key.to_lowercase(), value))
        .collect()
}

fn match_form_data(rules: &[FormDataItem], body_text: &str) -> bool {
    let enabled_rules: Vec<&FormDataItem> = rules.iter().filter(|rule| rule.enabled).collect();
    if enabled_rules.is_empty() {
        return true;
    }
    enabled_rules.into_iter().all(|rule| {
        let key = rule.key.trim();
        if key.is_empty() {
            return true;
        }
        let name_token = format!("name=\"{}\"", key);
        if !body_text.contains(&name_token) {
            return false;
        }
        let value = rule.value.trim();
        if value.is_empty() {
            return true;
        }
        match rule.field_type {
            FormDataFieldType::File => {
                let filename_token = format!("filename=\"{}\"", value);
                body_text.contains(&filename_token)
            }
            FormDataFieldType::Text => body_text.contains(value),
        }
    })
}

fn match_json_body(rule: &MockBodyMatch, body_text: &str) -> Option<bool> {
    if rule.mode == MatchMode::Regex {
        return None;
    }

    let rule_value = rule.value.trim();
    let body_trim = body_text.trim();
    if !is_json_like(rule_value) || !is_json_like(body_trim) {
        return None;
    }

    let rule_json = parse_json_with_comments(rule_value)?;
    let body_json = parse_json_with_comments(body_trim)?;

    let matched = match rule.mode {
        MatchMode::Exact => {
            if rule.validation_strategy == ValidationStrategy::KeyOnly {
                json_keys_match(&body_json, &rule_json)
            } else {
                body_json == rule_json
            }
        }
        MatchMode::Contains => {
            if rule.validation_strategy == ValidationStrategy::KeyOnly {
                json_keys_match(&body_json, &rule_json)
            } else {
                json_contains(&body_json, &rule_json)
            }
        }
        MatchMode::Regex => false,
    };

    Some(matched)
}

fn is_json_like(input: &str) -> bool {
    let trimmed = input.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn parse_json_with_comments(input: &str) -> Option<JsonValue> {
    serde_json::from_str::<JsonValue>(input)
        .ok()
        .or_else(|| serde_json::from_str::<JsonValue>(&strip_json_comments(input)).ok())
}

fn strip_json_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if in_string {
            output.push(c);
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            continue;
        }

        if c == '"' {
            in_string = true;
            output.push(c);
            continue;
        }

        if c == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    while let Some(nc) = chars.next() {
                        if nc == '\n' {
                            output.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut prev = '\0';
                    while let Some(nc) = chars.next() {
                        if prev == '*' && nc == '/' {
                            break;
                        }
                        prev = nc;
                    }
                    continue;
                }
                _ => {}
            }
        }

        output.push(c);
    }

    output
}

fn json_contains(haystack: &JsonValue, needle: &JsonValue) -> bool {
    match (haystack, needle) {
        (JsonValue::Object(hay), JsonValue::Object(need)) => need
            .iter()
            .all(|(k, v)| hay.get(k).map(|hv| json_contains(hv, v)).unwrap_or(false)),
        (JsonValue::Array(hay), JsonValue::Array(need)) => {
            if need.is_empty() {
                return true;
            }
            let mut used = vec![false; hay.len()];
            need.iter().all(|needle_item| {
                hay.iter().enumerate().any(|(idx, hay_item)| {
                    if used[idx] {
                        return false;
                    }
                    if json_contains(hay_item, needle_item) {
                        used[idx] = true;
                        true
                    } else {
                        false
                    }
                })
            })
        }
        _ => haystack == needle,
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

/// Find the matching response for multi-response mode
fn find_matching_response<'a>(route: &'a MockRoute, body_text: &str, match_mode: MultiResponseMatchMode) -> Option<&'a MockResponse> {
    match match_mode {
        MultiResponseMatchMode::Exact => {
            // Exact payload matching (original behavior)
            for mapping in &route.multi_responses {
                if match_payload_exact(&mapping.payload, body_text) {
                    return Some(&mapping.response);
                }
            }
            None
        }
        MultiResponseMatchMode::KeyMatch => {
            // Key-based matching: find mapping with matching keys
            find_matching_response_by_keys(route, body_text)
        }
    }
}

/// Find matching response based on JSON keys
/// Extracts keys from request body and finds first mapping with matching keys
fn find_matching_response_by_keys<'a>(route: &'a MockRoute, body_text: &str) -> Option<&'a MockResponse> {
    // Parse the request body to get its keys
    let request_json = parse_json_with_comments(body_text.trim())?;
    let request_keys = get_json_keys(&request_json);

    // Find first mapping where keys match
    for mapping in &route.multi_responses {
        let mapping_json = parse_json_with_comments(mapping.payload.trim())?;
        let mapping_keys = get_json_keys(&mapping_json);

        // Check if keys match (exact match)
        if request_keys == mapping_keys {
            return Some(&mapping.response);
        }
    }
    None
}

/// Extract top-level keys from a JSON value
fn get_json_keys(json: &JsonValue) -> Vec<String> {
    match json {
        JsonValue::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort(); // Sort for consistent comparison
            keys
        }
        _ => Vec::new(),
    }
}

/// Exact JSON payload matching (ignoring whitespace)
fn match_payload_exact(expected: &str, actual: &str) -> bool {
    // Parse both as JSON to normalize whitespace
    let expected_json = parse_json_with_comments(expected.trim());
    let actual_json = parse_json_with_comments(actual.trim());

    match (expected_json, actual_json) {
        (Some(exp), Some(act)) => exp == act,
        (None, None) => expected.trim() == actual.trim(),
        _ => false,
    }
}
