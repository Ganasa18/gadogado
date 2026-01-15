use crate::domain::error::{AppError, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::sync::Arc;

pub struct WebCrawler {
    client: Client,
    max_pages: usize,
    max_depth: usize,
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
        
        while !queue.is_empty() && results.len() < self.max_pages {
            let (url, depth) = queue.remove(0);
            
            if visited.contains(&url) || depth > self.max_depth {
                continue;
            }
            
            visited.insert(url.clone());
            
            match self.crawl_page(&url, &base_url).await {
                Ok(page) => {
                    add_log(
                        &logs,
                        "INFO",
                        "WebCrawler",
                        &format!("Crawled: {} ({} chars, {} links)", url, page.content.len(), page.links.len()),
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
    
    fn extract_links(&self, document: &Html, base_url: &str, current_url: &str) -> Result<Vec<String>> {
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
    
    fn normalize_link(&self, href: &str, base_url: &str, current_url: &str) -> Result<Option<String>> {
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
        let url_parsed = url::Url::parse(url)
            .map_err(|e| AppError::Internal(format!("Invalid URL: {}", e)))?;
        
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

#[derive(Debug, Clone)]
pub struct CrawledPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub links: Vec<String>,
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
        
        let result = crawler.normalize_link("/page", "https://example.com", "https://example.com/index").unwrap();
        assert_eq!(result, Some("https://example.com/page".to_string()));
        
        let result = crawler.normalize_link("https://example.com/other", "https://example.com", "https://example.com/index").unwrap();
        assert_eq!(result, Some("https://example.com/other".to_string()));
        
        let result = crawler.normalize_link("https://other.com/page", "https://example.com", "https://example.com/index").unwrap();
        assert_eq!(result, None);
    }
}
