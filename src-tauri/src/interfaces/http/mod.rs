use crate::domain::llm_config::LLMConfig;
use crate::domain::typegen::TypeGenMode;
use crate::interfaces::tauri::AppState;
use actix_cors::Cors;
use actix_web::{dev::Server, get, post, web, App, HttpResponse, HttpServer, Responder};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

pub struct HttpState {
    pub tauri_state: Arc<AppState>,
    pub logs: Arc<Mutex<Vec<LogEntry>>>,
}

#[derive(Deserialize)]
pub struct TranslateRequest {
    pub config: LLMConfig,
    pub content: String,
    pub source: String,
    pub target: String,
}

#[derive(Deserialize)]
pub struct EnhanceRequest {
    pub config: LLMConfig,
    pub content: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Deserialize)]
pub struct TypeGenRequest {
    pub config: LLMConfig,
    pub json: String,
    pub language: String,
    pub root_name: String,
    #[serde(default)]
    pub mode: TypeGenMode,
}

#[derive(Serialize)]
pub struct TypeGenResponse {
    pub result: String,
}

#[post("/translate")]
async fn translate(data: web::Data<HttpState>, req: web::Json<TranslateRequest>) -> impl Responder {
    add_log(
        &data.logs,
        "INFO",
        "HttpApi",
        &format!(
            "Translating: {} -> {} (provider={:?} base_url={})",
            req.source, req.target, req.config.provider, req.config.base_url
        ),
    );

    match data
        .tauri_state
        .translate_use_case
        .execute(
            &req.req_data().config,
            req.req_data().content.clone(),
            req.req_data().source.clone(),
            req.req_data().target.clone(),
        )
        .await
    {
        Ok(prompt) => HttpResponse::Ok().json(prompt),
        Err(e) => {
            add_log(
                &data.logs,
                "ERROR",
                "HttpApi",
                &format!("Translation failed: {}", e),
            );
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[post("/enhance")]
async fn enhance(data: web::Data<HttpState>, req: web::Json<EnhanceRequest>) -> impl Responder {
    add_log(
        &data.logs,
        "INFO",
        "HttpApi",
        &format!(
            "Enhancing prompt (provider={:?} base_url={})",
            req.config.provider, req.config.base_url
        ),
    );

    match data
        .tauri_state
        .enhance_use_case
        .execute(
            &req.req_data().config,
            req.req_data().content.clone(),
            req.req_data().system_prompt.clone(),
        )
        .await
    {
        Ok(prompt) => HttpResponse::Ok().json(prompt),
        Err(e) => {
            add_log(
                &data.logs,
                "ERROR",
                "HttpApi",
                &format!("Enhancement failed: {}", e),
            );
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[post("/typegen")]
async fn typegen(data: web::Data<HttpState>, req: web::Json<TypeGenRequest>) -> impl Responder {
    add_log(
        &data.logs,
        "INFO",
        "HttpApi",
        &format!(
            "Generating types (language={} mode={:?} provider={:?} base_url={})",
            req.language, req.mode, req.config.provider, req.config.base_url
        ),
    );

    match data
        .tauri_state
        .typegen_use_case
        .execute(
            &req.req_data().config,
            req.req_data().json.clone(),
            req.req_data().language.clone(),
            req.req_data().root_name.clone(),
            req.req_data().mode,
        )
        .await
    {
        Ok(result) => HttpResponse::Ok().json(TypeGenResponse { result }),
        Err(e) => {
            add_log(
                &data.logs,
                "ERROR",
                "HttpApi",
                &format!("Type generation failed: {}", e),
            );
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[post("/models")]
async fn list_models(data: web::Data<HttpState>, config: web::Json<LLMConfig>) -> impl Responder {
    add_log(
        &data.logs,
        "INFO",
        "HttpApi",
        &format!(
            "Fetching models (provider={:?} base_url={})",
            config.provider, config.base_url
        ),
    );

    match data.tauri_state.llm_client.list_models(&config).await {
        Ok(models) => HttpResponse::Ok().json(models),
        Err(e) => {
            add_log(
                &data.logs,
                "ERROR",
                "HttpApi",
                &format!("Failed to list models: {}", e),
            );
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

async fn fetch_openrouter_list(
    config: &LLMConfig,
    path: &str,
) -> std::result::Result<Vec<serde_json::Value>, String> {
    let base_url = config.base_url.trim_end_matches('/');
    if base_url.is_empty() {
        return Err("OpenRouter base_url is empty".to_string());
    }
    let url = format!("{}/{}", base_url, path);

    let mut request = reqwest::Client::new().get(&url);
    if let Some(api_key) = &config.api_key {
        request = request.bearer_auth(api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API error ({}): {}", status, text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let data = json["data"]
        .as_array()
        .ok_or_else(|| "Invalid response format: missing data array".to_string())?;

    Ok(data.iter().cloned().collect())
}

#[post("/openrouter/providers")]
async fn openrouter_providers(
    data: web::Data<HttpState>,
    config: web::Json<LLMConfig>,
) -> impl Responder {
    add_log(&data.logs, "INFO", "OpenRouter", "Fetching providers");

    match fetch_openrouter_list(&config, "providers").await {
        Ok(providers) => HttpResponse::Ok().json(providers),
        Err(err) => {
            add_log(
                &data.logs,
                "ERROR",
                "OpenRouter",
                &format!("Failed to fetch providers: {}", err),
            );
            HttpResponse::InternalServerError().body(err)
        }
    }
}

#[post("/openrouter/models")]
async fn openrouter_models(
    data: web::Data<HttpState>,
    config: web::Json<LLMConfig>,
) -> impl Responder {
    add_log(&data.logs, "INFO", "OpenRouter", "Fetching models");

    match fetch_openrouter_list(&config, "models").await {
        Ok(models) => HttpResponse::Ok().json(models),
        Err(err) => {
            add_log(
                &data.logs,
                "ERROR",
                "OpenRouter",
                &format!("Failed to fetch models: {}", err),
            );
            HttpResponse::InternalServerError().body(err)
        }
    }
}

#[get("/logs")]
async fn get_logs(data: web::Data<HttpState>) -> impl Responder {
    let logs = data.logs.lock().unwrap();
    HttpResponse::Ok().json(&*logs)
}

#[derive(Deserialize)]
struct ProxyQuery {
    url: String,
}

#[get("/qa/proxy")]
async fn qa_proxy(data: web::Data<HttpState>, query: web::Query<ProxyQuery>) -> impl Responder {
    let target_url = &query.url;

    add_log(
        &data.logs,
        "INFO",
        "QA Proxy",
        &format!("Proxying request to: {}", target_url),
    );

    // Fetch the HTML from target URL
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .danger_accept_invalid_certs(true) // For localhost development
        .build()
        .unwrap();

    match client.get(target_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                add_log(
                    &data.logs,
                    "ERROR",
                    "QA Proxy",
                    &format!("Failed to fetch URL: HTTP {}", response.status()),
                );
                return HttpResponse::BadGateway()
                    .insert_header(("Access-Control-Allow-Origin", "*"))
                    .body(format!("Failed to fetch URL: HTTP {}", response.status()));
            }

            match response.text().await {
                Ok(html) => {
                    // Inject the recorder script into the HTML
                    let injected_html = inject_recorder_script(&html, target_url);

                    add_log(
                        &data.logs,
                        "INFO",
                        "QA Proxy",
                        "Successfully proxied and injected recorder script",
                    );

                    HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .insert_header(("Access-Control-Allow-Origin", "*"))
                        .insert_header(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
                        .insert_header(("Access-Control-Allow-Headers", "*"))
                        .insert_header(("X-Frame-Options", "ALLOWALL"))
                        .body(injected_html)
                }
                Err(e) => {
                    add_log(
                        &data.logs,
                        "ERROR",
                        "QA Proxy",
                        &format!("Failed to read response body: {}", e),
                    );
                    HttpResponse::InternalServerError()
                        .insert_header(("Access-Control-Allow-Origin", "*"))
                        .body(format!("Failed to read response: {}", e))
                }
            }
        }
        Err(e) => {
            add_log(
                &data.logs,
                "ERROR",
                "QA Proxy",
                &format!("Failed to fetch URL: {}", e),
            );
            HttpResponse::BadGateway()
                .insert_header(("Access-Control-Allow-Origin", "*"))
                .body(format!("Failed to fetch URL: {}", e))
        }
    }
}

fn inject_recorder_script(html: &str, base_url: &str) -> String {
    // Parse base URL to extract origin for proper script/resource loading
    let _origin = if let Ok(url) = url::Url::parse(base_url) {
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
    } else {
        "http://localhost:1420".to_string()
    };

    // Use the origin as base href for proper resource loading
    // let base_href = format!("{}/", origin);

    // Inject recorder script and base tag with CSP bypass
    let injection = format!(
        r#"<base href=\"{}\">
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src * 'unsafe-inline' 'unsafe-eval' data: blob:;\">
<script>
// QA Recorder Injectable Script - Injected by Proxy
(function() {{
  'use strict';

  const INPUT_DEBOUNCE_MS = 350;
  const MAX_TEXT_LENGTH = 160;

  let inputTimers = new Map();
  let lastPointer = null;
  let lastFocusedElement = null;

  if (window.__QA_RECORDER_INJECTED__) {{
    return;
  }}
  window.__QA_RECORDER_INJECTED__ = true;

  console.log('[QA Recorder Inject] Script loaded via proxy');

  function postEventToParent(payload) {{
    window.parent.postMessage({{
      type: 'qa-recorder-event',
      payload: payload
    }}, '*');
  }}

  function buildSelector(element) {{
    const prioritized = ['data-testid', 'data-purpose', 'id', 'name', 'aria-label', 'role'];
    for (const attr of prioritized) {{
      const value = element.getAttribute(attr);
      if (!value) continue;
      if (attr === 'id') return `#${{CSS.escape(value)}}`;
      return `${{element.tagName.toLowerCase()}}[${{attr}}=\"${{CSS.escape(value)}}\"]`;
    }}
    const path = [];
    let current = element;
    const rootBody = element.ownerDocument?.body;
    for (let depth = 0; current && current !== rootBody && depth < 4; depth++) {{
      const tagName = current.tagName.toLowerCase();
      const index = nthOfType(current);
      path.unshift(`${{tagName}}:nth-of-type(${{index}})`);
      current = current.parentElement;
    }}
    return path.length > 0 ? path.join(' > ') : undefined;
  }}

  function nthOfType(element) {{
    let index = 1;
    let sibling = element.previousElementSibling;
    while (sibling) {{
      if (sibling.tagName === element.tagName) index++;
      sibling = sibling.previousElementSibling;
    }}
    return index;
  }}

  function getElementText(element) {{
    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {{
      return normalizeText(element.getAttribute('aria-label') || element.placeholder || element.name);
    }}
    if (element instanceof HTMLSelectElement) {{
      const selected = element.selectedOptions?.[0]?.textContent;
      return normalizeText(selected || element.getAttribute('aria-label') || element.name);
    }}
    return normalizeText(element.textContent);
  }}

  function getElementValue(element) {{
    if (element instanceof HTMLInputElement) return element.value;
    if (element instanceof HTMLTextAreaElement) return element.value;
    if (element instanceof HTMLSelectElement) return element.value;
    if (element instanceof HTMLElement && element.isContentEditable) return element.innerText;
    return undefined;
  }}

  function maskValue(element, value) {{
    if (!value || value.trim().length === 0) return undefined;
    if (element instanceof HTMLInputElement && element.type === 'password') return '[masked]';
    const label = [element.getAttribute('name'), element.getAttribute('id'), element.getAttribute('aria-label')]
      .filter(Boolean).join(' ').toLowerCase();
    if (label.includes('password')) return '[masked]';
    return value;
  }}

  function normalizeText(value) {{
    if (!value) return undefined;
    const trimmed = value.trim();
    if (!trimmed) return undefined;
    return trimmed.length > MAX_TEXT_LENGTH ? trimmed.slice(0, MAX_TEXT_LENGTH) : trimmed;
  }}

  function stringifyMeta(meta) {{
    const cleaned = {{}};
    Object.entries(meta).forEach(([key, value]) => {{
      if (value === undefined || value === null || value === '') return;
      cleaned[key] = value;
    }});
    return Object.keys(cleaned).length > 0 ? JSON.stringify(cleaned) : undefined;
  }}

  function getCoordinates(event) {{
    if (event instanceof MouseEvent) return {{ x: event.clientX, y: event.clientY }};
    return lastPointer ?? undefined;
  }}

  function handlePointerDown(event) {{
    const coords = getCoordinates(event);
    if (coords) lastPointer = coords;
  }}

  function handleClick(event) {{
    const target = event.target;
    if (!(target instanceof Element)) return;
    const isEditable =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable);
    postEventToParent({{
      eventType: 'click',
      selector: buildSelector(target),
      elementText: getElementText(target),
      url: window.location.href,
      metaJson: stringifyMeta({{
        tag: target.tagName.toLowerCase(),
        type: target instanceof HTMLInputElement ? target.type : undefined,
        isEditable,
        coordinates: getCoordinates(event)
      }}),
    }});
  }}

  function handleInput(event) {{
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement)) return;
    const inputType = event.inputType;
    const previousTimer = inputTimers.get(target);
    if (previousTimer) clearTimeout(previousTimer);
    const nextTimer = setTimeout(() => {{
      inputTimers.delete(target);
      const rawValue = getElementValue(target);
      const maskedValue = maskValue(target, rawValue);
      postEventToParent({{
        eventType: 'input',
        selector: buildSelector(target),
        elementText: getElementText(target),
        value: maskedValue,
        url: window.location.href,
        metaJson: stringifyMeta({{
          tag: target.tagName.toLowerCase(),
          inputType,
          type: target instanceof HTMLInputElement ? target.type : undefined,
          coordinates: getCoordinates(event),
        }}),
      }});
    }}, INPUT_DEBOUNCE_MS);
    inputTimers.set(target, nextTimer);
  }}

  function handleSubmit(event) {{
    const target = event.target;
    if (!(target instanceof Element)) return;
    const form = target instanceof HTMLFormElement ? target : target.closest('form');
    const element = form ?? target;
    postEventToParent({{
      eventType: 'submit',
      selector: buildSelector(element),
      elementText: getElementText(element),
      url: window.location.href,
      metaJson: stringifyMeta({{
        tag: element.tagName.toLowerCase(),
        action: form?.action,
        method: form?.method,
        coordinates: getCoordinates(event),
      }}),
    }});
  }}

  document.addEventListener('pointerdown', handlePointerDown, true);
  document.addEventListener('click', handleClick, true);
  document.addEventListener('input', handleInput, true);
  document.addEventListener('submit', handleSubmit, true);
  document.addEventListener(
    'focusin',
    (event) => {{
      if (event.target instanceof Element) {{
        lastFocusedElement = event.target;
      }}
    }},
    true
  );

  function handleParentCommand(event) {{
    if (event.source !== window.parent) return;
    if (!event.data || event.data.type !== 'qa-recorder-command') return;

    const action = event.data.action;
    if (action === 'back') {{
      window.history.back();
      return;
    }}
    if (action === 'refocus') {{
      if (lastFocusedElement && document.contains(lastFocusedElement)) {{
        if (typeof lastFocusedElement.focus === 'function') {{
          lastFocusedElement.focus({{ preventScroll: true }});
        }}
      }}
      return;
    }}
    if (action === 'capture') {{
      const requestId = event.data.requestId;
      captureDocumentAsDataUrl()
        .then((dataUrl) => {{
          window.parent.postMessage(
            {{ type: 'qa-recorder-capture', requestId, dataUrl }},
            '*'
          );
        }})
        .catch((err) => {{
          window.parent.postMessage(
            {{
              type: 'qa-recorder-capture-error',
              requestId,
              error: err?.message || 'Failed to capture preview.',
            }},
            '*'
          );
        }});
    }}
  }}

  async function captureDocumentAsDataUrl() {{
    try {{
      return await renderDocumentToDataUrl(document.documentElement);
    }} catch (err) {{
      if (isTaintedCanvasError(err)) {{
        const sanitized = sanitizeDocumentElement(document.documentElement);
        return renderDocumentToDataUrl(sanitized);
      }}
      throw err;
    }}
  }}

  function isTaintedCanvasError(err) {{
    const message = err?.message || '';
    return (
      message.includes('Tainted canvases') ||
      message.includes('SecurityError')
    );
  }}

  function sanitizeDocumentElement(root) {{
    const clone = root.cloneNode(true);
    const stripSelectors = [
      'img',
      'picture',
      'source',
      'video',
      'audio',
      'canvas',
      'iframe',
      'svg',
      'link[rel="stylesheet"]',
    ];
    clone.querySelectorAll(stripSelectors.join(',')).forEach((el) => el.remove());

    clone.querySelectorAll('style').forEach((style) => {{
      if (!style.textContent) return;
      let text = style.textContent;
      text = text.replace(/@font-face\s*\{{[\s\S]*?\}}/g, '');
      text = text.replace(/url\(([^)]+)\)/g, 'none');
      style.textContent = text;
    }});

    clone.querySelectorAll('[style]').forEach((el) => {{
      const inline = el.getAttribute('style');
      if (!inline || !inline.includes('url(')) return;
      const cleaned = inline.replace(/url\(([^)]+)\)/g, 'none');
      el.setAttribute('style', cleaned);
    }});

    return clone;
  }}

  async function renderDocumentToDataUrl(root) {{
    const safeWidth = Math.max(1, Math.floor(window.innerWidth));
    const safeHeight = Math.max(1, Math.floor(window.innerHeight));
    const serialized = new XMLSerializer().serializeToString(root);
    const wrapped = `<div xmlns=\"http://www.w3.org/1999/xhtml\">${{serialized}}</div>`;
    const svg = `<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"${{safeWidth}}\" height=\"${{safeHeight}}\"><foreignObject width=\"100%\" height=\"100%\">${{wrapped}}</foreignObject></svg>`;
    const blob = new Blob([svg], {{ type: 'image/svg+xml;charset=utf-8' }});
    const url = URL.createObjectURL(blob);

    try {{
      const img = await new Promise((resolve, reject) => {{
        const image = new Image();
        image.onload = () => resolve(image);
        image.onerror = () => reject(new Error('Failed to render preview snapshot.'));
        image.src = url;
      }});

      const canvas = document.createElement('canvas');
      canvas.width = safeWidth;
      canvas.height = safeHeight;
      const ctx = canvas.getContext('2d');
      if (!ctx) {{
        throw new Error('Canvas is not available for screenshot.');
      }}
      ctx.drawImage(img, 0, 0, safeWidth, safeHeight);
      return canvas.toDataURL('image/png');
    }} finally {{
      URL.revokeObjectURL(url);
    }}
  }}

  window.addEventListener('message', handleParentCommand);

  window.parent.postMessage({{ type: 'qa-recorder-ready' }}, '*');
  console.log('[QA Recorder Inject] Ready');
}})();
</script>
"#,
        base_url
    );

    // Try to inject right after <head> tag, fallback to before </head>, finally before <body>
    if let Some(pos) = html.find("<head>") {
        let insert_pos = pos + "<head>".len();
        format!(
            "{}{}{}",
            &html[..insert_pos],
            injection,
            &html[insert_pos..]
        )
    } else if let Some(pos) = html.find("</head>") {
        format!("{}{}{}", &html[..pos], injection, &html[pos..])
    } else if let Some(pos) = html.find("<body") {
        format!("{}{}{}", &html[..pos], injection, &html[pos..])
    } else {
        // Fallback: prepend to entire HTML
        format!("{}{}", injection, html)
    }
}

pub fn add_log_entry(
    logs: &Mutex<Vec<LogEntry>>,
    level: &str,
    source: &str,
    message: &str,
) -> LogEntry {
    let entry = LogEntry {
        time: Local::now().format("%H:%M:%S").to_string(),
        level: level.to_string(),
        source: source.to_string(),
        message: message.to_string(),
    };
    let mut logs = logs.lock().unwrap();
    logs.push(entry.clone());
    if logs.len() > 100 {
        logs.remove(0);
    }
    entry
}

pub fn add_log(logs: &Mutex<Vec<LogEntry>>, level: &str, source: &str, message: &str) {
    add_log_entry(logs, level, source, message);
}

pub fn start_server(
    tauri_state: Arc<AppState>,
    logs: Arc<Mutex<Vec<LogEntry>>>,
) -> std::io::Result<Server> {
    let state = web::Data::new(HttpState { tauri_state, logs });

    let server = HttpServer::new(move || {
        let cors = Cors::permissive(); // Allow all origins for local tool

        App::new().wrap(cors).app_data(state.clone()).service(
            web::scope("/api")
                .service(translate)
                .service(enhance)
                .service(typegen)
                .service(list_models)
                .service(openrouter_providers)
                .service(openrouter_models)
                .service(get_logs)
                .service(qa_proxy),
        )
    })
    .bind(("127.0.0.1", 3001))?
    .run();

    Ok(server)
}

// Helper trait to avoid move issues in handlers
trait RequestData<T> {
    fn req_data(&self) -> &T;
}

impl<T> RequestData<T> for web::Json<T> {
    fn req_data(&self) -> &T {
        &**self
    }
}
