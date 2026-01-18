use crate::domain::error::{AppError, Result};
use crate::infrastructure::playwright::{CaptureManifest, PlaywrightCapture};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::sleep;

pub struct WebCrawler {
    client: Client,
    max_pages: usize,
    max_depth: usize,
}

#[derive(Debug, Clone, Default)]
struct RobotsRules {
    disallow_paths: Vec<String>,
    crawl_delay: Option<std::time::Duration>,
}

impl WebCrawler {
    pub fn new(max_pages: usize, max_depth: usize) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Mozilla/5.0 (compatible; LocalSenseRAG/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            max_pages,
            max_depth,
        }
    }

    pub fn with_config(max_pages: usize, max_depth: usize, timeout_secs: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (compatible; LocalSenseRAG/1.0)")
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            max_pages,
            max_depth,
        })
    }

    pub async fn crawl_site(
        &self,
        start_url: &str,
        logs: Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<Vec<CrawledPage>> {
        use crate::interfaces::http::add_log;

        let base_url = self.extract_base_url(start_url)?;
        let mut visited = HashSet::new();
        let mut results = Vec::new();
        let mut queue = vec![(start_url.to_string(), 0)];

        add_log(
            &logs,
            "INFO",
            "WebCrawler",
            &format!("Starting crawl: {}", start_url),
        );

        let robots = self.fetch_robots_rules(&base_url, logs.clone()).await;
        let request_delay = robots
            .as_ref()
            .and_then(|rules| rules.crawl_delay)
            .unwrap_or_else(|| std::time::Duration::from_secs(1));

        if let Some(rules) = &robots {
            add_log(
                &logs,
                "INFO",
                "WebCrawler",
                &format!(
                    "Robots rules: {} disallow entries, delay {:?}",
                    rules.disallow_paths.len(),
                    request_delay
                ),
            );
        } else {
            add_log(
                &logs,
                "INFO",
                "WebCrawler",
                "Robots.txt not found or unavailable; proceeding with polite defaults",
            );
        }

        while !queue.is_empty() && results.len() < self.max_pages {
            let (url, depth) = queue.remove(0);

            if visited.contains(&url) || depth > self.max_depth {
                continue;
            }

            if let Some(rules) = &robots {
                if self.is_disallowed_by_robots(&url, &base_url, rules) {
                    add_log(
                        &logs,
                        "INFO",
                        "WebCrawler",
                        &format!("Skipping {} due to robots.txt", url),
                    );
                    continue;
                }
            }

            visited.insert(url.clone());
            sleep(request_delay).await;

            match self.crawl_page(&url, &base_url).await {
                Ok(page) => {
                    add_log(
                        &logs,
                        "INFO",
                        "WebCrawler",
                        &format!(
                            "Crawled: {} ({} chars, {} links)",
                            url,
                            page.content.len(),
                            page.links.len()
                        ),
                    );

                    for link in &page.links {
                        if !visited.contains(link) && queue.len() < self.max_pages * 2 {
                            queue.push((link.clone(), depth + 1));
                        }
                    }

                    results.push(page);
                }
                Err(e) => {
                    add_log(
                        &logs,
                        "WARN",
                        "WebCrawler",
                        &format!("Failed to crawl {}: {}", url, e),
                    );
                }
            }
        }

        add_log(
            &logs,
            "INFO",
            "WebCrawler",
            &format!("Crawl complete: {} pages visited", results.len()),
        );

        Ok(results)
    }

    async fn fetch_robots_rules(
        &self,
        base_url: &str,
        logs: Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Option<RobotsRules> {
        use crate::interfaces::http::add_log;

        let robots_url = format!("{}/robots.txt", base_url.trim_end_matches('/'));
        let response = self.client.get(&robots_url).send().await.ok()?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return None;
        }

        if !response.status().is_success() {
            add_log(
                &logs,
                "WARN",
                "WebCrawler",
                &format!(
                    "Failed to fetch robots.txt ({}): {}",
                    robots_url,
                    response.status()
                ),
            );
            return None;
        }

        let body = response.text().await.ok()?;
        Some(Self::parse_robots_rules(&body))
    }

    fn parse_robots_rules(body: &str) -> RobotsRules {
        let mut rules = RobotsRules::default();
        let mut applies_to_us = false;

        for raw_line in body.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.splitn(2, ':');
            let directive = parts.next().unwrap_or("").trim().to_lowercase();
            let value = parts.next().unwrap_or("").trim();

            match directive.as_str() {
                "user-agent" => {
                    applies_to_us = value == "*";
                }
                "disallow" if applies_to_us => {
                    if !value.is_empty() {
                        rules.disallow_paths.push(value.to_string());
                    }
                }
                "crawl-delay" if applies_to_us => {
                    if let Ok(delay) = value.parse::<f64>() {
                        let millis = (delay * 1000.0) as u64;
                        rules.crawl_delay = Some(std::time::Duration::from_millis(millis));
                    }
                }
                _ => {}
            }
        }

        rules
    }

    fn is_disallowed_by_robots(&self, url: &str, base_url: &str, rules: &RobotsRules) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let base_path = parsed.path();
        let base_prefix = base_url.trim_end_matches('/');

        if !url.starts_with(base_prefix) {
            return true;
        }

        rules.disallow_paths.iter().any(|prefix| {
            if prefix == "/" {
                return true;
            }
            base_path.starts_with(prefix)
        })
    }

    async fn crawl_page(&self, url: &str, base_url: &str) -> Result<CrawledPage> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch URL: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "HTTP error {}: {}",
                response.status(),
                url
            )));
        }

        let html = response
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read response body: {}", e)))?;

        let document = Html::parse_document(&html);

        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| url.to_string());

        let body_text = document
            .select(&Selector::parse("body").unwrap())
            .next()
            .map(|el| {
                el.text()
                    .collect::<Vec<_>>()
                    .join("\n")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(String::new);

        let links = self.extract_links(&document, base_url, url)?;

        let content = if title != url {
            format!("{}\n\n{}", title, body_text)
        } else {
            body_text
        };

        Ok(CrawledPage {
            url: url.to_string(),
            title,
            content,
            links,
        })
    }

    fn extract_links(
        &self,
        document: &Html,
        base_url: &str,
        current_url: &str,
    ) -> Result<Vec<String>> {
        let mut links = Vec::new();
        let anchor_selector = Selector::parse("a[href]").unwrap();

        for element in document.select(&anchor_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Some(normalized) = self.normalize_link(href, base_url, current_url)? {
                    if !links.contains(&normalized) {
                        links.push(normalized);
                    }
                }
            }
        }

        Ok(links)
    }

    fn normalize_link(
        &self,
        href: &str,
        base_url: &str,
        current_url: &str,
    ) -> Result<Option<String>> {
        let href = href.trim();

        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            return Ok(None);
        }

        if href.starts_with("http://") || href.starts_with("https://") {
            if href.starts_with(base_url) {
                Ok(Some(href.to_string()))
            } else {
                Ok(None)
            }
        } else if href.starts_with('/') {
            Ok(Some(format!("{}{}", base_url, href)))
        } else {
            let current_base = if let Some(pos) = current_url.rfind('/') {
                &current_url[..pos]
            } else {
                current_url
            };
            Ok(Some(format!("{}/{}", current_base, href)))
        }
    }

    fn extract_base_url(&self, url: &str) -> Result<String> {
        let url_parsed =
            url::Url::parse(url).map_err(|e| AppError::Internal(format!("Invalid URL: {}", e)))?;

        let base = format!(
            "{}://{}",
            url_parsed.scheme(),
            url_parsed.host_str().unwrap_or("")
        );

        Ok(base)
    }

    pub fn clean_html(html: &str) -> Result<String> {
        let document = Html::parse_document(html);

        let body_text = document
            .select(&Selector::parse("body").unwrap())
            .next()
            .map(|el| {
                el.text()
                    .collect::<Vec<_>>()
                    .join("\n")
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| {
                Html::parse_document(html)
                    .root_element()
                    .text()
                    .collect::<Vec<_>>()
                    .join("\n")
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            });

        Ok(body_text)
    }
}

/// Web crawl mode: HTML parsing (fast) or Screenshot OCR (accurate for JS-heavy sites)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebCrawlMode {
    /// Use reqwest + scraper for fast HTML extraction
    Html,
    /// Use Playwright screenshots + Tesseract OCR for JS-heavy sites
    Ocr,
}

impl Default for WebCrawlMode {
    fn default() -> Self {
        WebCrawlMode::Html
    }
}

#[derive(Debug, Clone)]
pub struct CrawledPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub links: Vec<String>,
}

/// Result from OCR-based web capture
#[derive(Debug, Clone)]
pub struct OcrCrawlResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub manifest: CaptureManifest,
    pub output_dir: PathBuf,
}

/// OCR web capture service
pub struct WebOcrCapture {
    playwright: PlaywrightCapture,
    temp_dir: PathBuf,
}

impl WebOcrCapture {
    /// Create a new WebOcrCapture with the given script path and temp directory
    pub fn new(script_path: PathBuf, temp_dir: PathBuf) -> Self {
        Self {
            playwright: PlaywrightCapture::new(script_path),
            temp_dir,
        }
    }

    /// Capture a single URL using screenshots and OCR
    pub async fn capture_url(
        &self,
        url: &str,
        logs: Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<OcrCrawlResult> {
        use crate::interfaces::http::add_log;

        // Create unique output directory for this capture
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let url_hash = Self::hash_url(url);
        let output_dir = self
            .temp_dir
            .join(format!("web_ocr_{}_{}", timestamp, url_hash));

        add_log(
            &logs,
            "INFO",
            "WebOcrCapture",
            &format!("Starting OCR capture: {}", url),
        );

        // Check Node.js availability
        match PlaywrightCapture::check_nodejs() {
            Ok(version) => {
                add_log(
                    &logs,
                    "DEBUG",
                    "WebOcrCapture",
                    &format!("Node.js version: {}", version),
                );
            }
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Node.js required for OCR capture: {}",
                    e
                )));
            }
        }

        // Create progress channel for logging
        let (tx, mut rx) = mpsc::channel(32);
        let logs_clone = logs.clone();

        // Spawn task to handle progress messages
        let progress_task = tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                use crate::infrastructure::playwright::CaptureProgress;
                let msg = match progress {
                    CaptureProgress::Navigating { url } => {
                        format!("Navigating to: {}", url)
                    }
                    CaptureProgress::Retrying { reason } => {
                        format!("Retrying: {}", reason)
                    }
                    CaptureProgress::Dimensions { width, height } => {
                        format!("Page dimensions: {}x{}", width, height)
                    }
                    CaptureProgress::Capturing { num_tiles } => {
                        format!("Capturing {} tiles...", num_tiles)
                    }
                    CaptureProgress::TileCaptured { index, total } => {
                        format!("Captured tile {}/{}", index + 1, total)
                    }
                    CaptureProgress::Complete { tiles_count, .. } => {
                        format!("Screenshot capture complete: {} tiles", tiles_count)
                    }
                };
                add_log(&logs_clone, "DEBUG", "WebOcrCapture", &msg);
            }
        });

        // Capture screenshots
        let manifest = self
            .playwright
            .capture_url(url, &output_dir, Some(tx))
            .await?;

        // Wait for progress logging to complete
        let _ = progress_task.await;

        add_log(
            &logs,
            "INFO",
            "WebOcrCapture",
            &format!("Captured {} tiles, starting OCR...", manifest.tiles.len()),
        );

        // Get tile paths and run OCR
        let tile_paths = self.playwright.get_tile_paths(&output_dir, &manifest);
        let content = self.ocr_tiles(&tile_paths, &logs).await?;

        add_log(
            &logs,
            "INFO",
            "WebOcrCapture",
            &format!("OCR complete: {} characters extracted", content.len()),
        );

        // Save output markdown
        let markdown_path = output_dir.join("out.md");
        let markdown_content = format!(
            "# {}\n\nSource: {}\n\n---\n\n{}",
            manifest.title, manifest.url, content
        );
        std::fs::write(&markdown_path, &markdown_content)
            .map_err(|e| AppError::Internal(format!("Failed to write markdown output: {}", e)))?;

        Ok(OcrCrawlResult {
            url: manifest.url.clone(),
            title: manifest.title.clone(),
            content,
            manifest,
            output_dir,
        })
    }

    /// Run OCR on captured tiles
    async fn ocr_tiles(
        &self,
        tile_paths: &[PathBuf],
        logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<String> {
        use crate::interfaces::http::add_log;
        use std::process::Command;

        let mut combined_text = String::new();
        let tesseract_cmd =
            std::env::var("TESSERACT_CMD").unwrap_or_else(|_| "tesseract".to_string());
        let tessdata_prefix = std::env::var("TESSDATA_PREFIX").ok();

        add_log(
            logs,
            "INFO",
            "WebOcrCapture",
            &format!("Using Tesseract command: {}", tesseract_cmd),
        );
        if let Some(prefix) = &tessdata_prefix {
            add_log(
                logs,
                "DEBUG",
                "WebOcrCapture",
                &format!("Using TESSDATA_PREFIX: {}", prefix),
            );
        }

        for (idx, tile_path) in tile_paths.iter().enumerate() {
            if !tile_path.exists() {
                add_log(
                    logs,
                    "WARN",
                    "WebOcrCapture",
                    &format!("Tile not found: {}", tile_path.display()),
                );
                continue;
            }

            // Run Tesseract OCR directly on the original image
            let mut command = Command::new(&tesseract_cmd);
            if let Some(prefix) = &tessdata_prefix {
                command.env("TESSDATA_PREFIX", prefix);
            }
            let output = command
                .arg(tile_path)
                .arg("stdout")
                .arg("-l")
                .arg("eng")
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    let text = String::from_utf8_lossy(&result.stdout).to_string();
                    if !text.trim().is_empty() {
                        combined_text.push_str(&format!("<!-- tile:{} -->\n", idx + 1));
                        combined_text.push_str(&text);
                        combined_text.push_str("\n\n");
                    }
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    add_log(
                        logs,
                        "WARN",
                        "WebOcrCapture",
                        &format!("Tesseract failed for tile {}: {}", idx + 1, stderr),
                    );
                }
                Err(e) => {
                    add_log(
                        logs,
                        "WARN",
                        "WebOcrCapture",
                        &format!("Failed to run Tesseract for tile {}: {}", idx + 1, e),
                    );
                }
            }

            add_log(
                logs,
                "DEBUG",
                "WebOcrCapture",
                &format!("OCR complete for tile {}/{}", idx + 1, tile_paths.len()),
            );
        }

        Ok(combined_text)
    }

    /// Generate a short hash of the URL for directory naming
    fn hash_url(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:x}", hasher.finish())[..8].to_string()
    }

    /// Clean up temporary files after processing
    pub fn cleanup(&self, output_dir: &Path) -> Result<()> {
        if output_dir.exists() {
            std::fs::remove_dir_all(output_dir).map_err(|e| {
                AppError::Internal(format!("Failed to clean up temp directory: {}", e))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html() {
        let html = r#"<html><head><title>Test</title></head><body><h1>Hello</h1><p>World</p></body></html>"#;
        let cleaned = WebCrawler::clean_html(html).unwrap();
        assert!(cleaned.contains("Hello"));
        assert!(cleaned.contains("World"));
    }

    #[test]
    fn test_normalize_link() {
        let crawler = WebCrawler::new(10, 2);

        let result = crawler
            .normalize_link("/page", "https://example.com", "https://example.com/index")
            .unwrap();
        assert_eq!(result, Some("https://example.com/page".to_string()));

        let result = crawler
            .normalize_link(
                "https://example.com/other",
                "https://example.com",
                "https://example.com/index",
            )
            .unwrap();
        assert_eq!(result, Some("https://example.com/other".to_string()));

        let result = crawler
            .normalize_link(
                "https://other.com/page",
                "https://example.com",
                "https://example.com/index",
            )
            .unwrap();
        assert_eq!(result, None);
    }
}
