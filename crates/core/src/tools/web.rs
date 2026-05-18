//! Web search and fetch tools for DeepSeek Mobile.
//!
//! These tools let the model fetch URLs and search the web from the mobile app
//! or through the PC gateway. Every network operation requires explicit approval
//! (Suggest level) since the app runs on a phone with mobile data.
//!
//! `web_fetch` — fetches a single URL and returns its text content.
//! `web_search` — submits a query to DuckDuckGo Lite and returns organic results.

use super::{required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::time::Duration;

/// Maximum response body size for web fetch (512 KB).
const MAX_FETCH_BYTES: usize = 512 * 1024;
/// Maximum response body size for search results (256 KB).
const MAX_SEARCH_BYTES: usize = 256 * 1024;
/// Default HTTP request timeout.
const HTTP_TIMEOUT_SECS: u64 = 30;

// ---------------------------------------------------------------------------
// web_fetch
// ---------------------------------------------------------------------------

pub struct WebFetchTool;

impl ToolSpec for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch a URL and return its plain-text content. Supports HTTP and HTTPS URLs. \
         Use this to read API documentation, web pages, or raw file content from the internet."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (http:// or https://)"
                },
                "max_bytes": {
                    "type": "integer",
                    "description": "Optional: maximum bytes to read (default 512 KB, max 512 KB)"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Optional: timeout in seconds (default 30, max 60)"
                }
            },
            "required": ["url"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Network]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult> {
        let url = required_str(&input, "url")?;
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("URL must start with http:// or https://: {}", url));
        }

        let max_bytes = input
            .get("max_bytes")
            .and_then(Value::as_u64)
            .map(|v| (v as usize).min(MAX_FETCH_BYTES))
            .unwrap_or(MAX_FETCH_BYTES);

        let timeout_secs = input
            .get("timeout_secs")
            .and_then(Value::as_u64)
            .map(|v| v.min(60))
            .unwrap_or(HTTP_TIMEOUT_SECS);

        let rt = tokio::runtime::Handle::current();
        let body = rt.block_on(fetch_url_text(url, max_bytes, timeout_secs))?;

        Ok(ToolResult::success(body).with_metadata(json!({
            "url": url,
            "source": "web_fetch"
        })))
    }
}

// ---------------------------------------------------------------------------
// web_search
// ---------------------------------------------------------------------------

pub struct WebSearchTool;

impl ToolSpec for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo Lite and return organic results with titles, \
         URLs and snippets. Use this to find documentation, tutorials, news, or any \
         information available on the internet."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Optional: maximum results to return (default 10, max 30)"
                }
            },
            "required": ["query"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Network]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult> {
        let query = required_str(&input, "query")?;
        let max_results = input
            .get("max_results")
            .and_then(Value::as_u64)
            .map(|v| (v as usize).min(30))
            .unwrap_or(10);

        let rt = tokio::runtime::Handle::current();
        let results = rt.block_on(search_duckduckgo(query, max_results))?;

        Ok(ToolResult::success(serde_json::to_string_pretty(&results)?)
            .with_metadata(json!({
                "query": query,
                "result_count": results.len(),
                "source": "web_search"
            })))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Fetch a URL and return plain text content.
async fn fetch_url_text(url: &str, max_bytes: usize, timeout_secs: u64) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("DeepSeek-Mobile/0.1")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| anyhow!("failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or("")));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| anyhow!("failed to read response body: {}", e))?;

    let body = if bytes.len() > max_bytes {
        let mut truncated = String::from_utf8_lossy(&bytes[..max_bytes]).to_string();
        truncated.push_str("\n\n... <response truncated at ");
        truncated.push_str(&max_bytes.to_string());
        truncated.push_str(" bytes>");
        truncated
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };

    let mut result = String::new();
    result.push_str(&format!("URL: {}\n", url));
    result.push_str(&format!("Status: {}\n", status));
    result.push_str(&format!("Content-Type: {}\n\n", content_type));
    result.push_str(&body);

    Ok(result)
}

/// Single DDG Lite result.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct DdgResult {
    title: String,
    url: String,
    snippet: String,
}

/// Search DuckDuckGo Lite and return parsed organic results.
async fn search_duckduckgo(query: &str, max_results: usize) -> Result<Vec<DdgResult>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .user_agent("DeepSeek-Mobile/0.1")
        .build()
        .map_err(|e| anyhow!("failed to build HTTP client: {}", e))?;

    let params = [("q", query)];
    let response = client
        .post("https://lite.duckduckgo.com/lite/")
        .form(&params)
        .send()
        .await
        .map_err(|e| anyhow!("search request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("search API returned HTTP {}", status.as_u16()));
    }

    let html = response
        .text()
        .await
        .map_err(|e| anyhow!("failed to read search response: {}", e))?;

    if html.len() > MAX_SEARCH_BYTES {
        return Err(anyhow!("search response too large ({} bytes)", html.len()));
    }

    // Parse the HTML table from DuckDuckGo Lite.
    // DDG Lite returns results in a <table> with class="result" — each row
    // contains a link (class="result-link") and a snippet (class="result-snippet").
    let results = parse_ddg_lite_html(&html, max_results);
    if results.is_empty() {
        return Err(anyhow!("no results found for query: {}", query));
    }

    Ok(results)
}

/// Minimal DuckDuckGo Lite HTML parser using basic string matching.
/// Avoids an HTML parser dependency.
fn parse_ddg_lite_html(html: &str, max_results: usize) -> Vec<DdgResult> {
    let mut results = Vec::new();
    let mut pos = 0;
    let html_bytes = html.as_bytes();

    while results.len() < max_results {
        // Find next result row
        let result_start = find_after(html_bytes, pos, "class=\"result\"");
        let result_start = match result_start {
            Some(p) => p,
            None => break,
        };

        // Find the title link: <a rel="nofollow" href="...">
        let href_start = find_after(html_bytes, result_start, "href=\"");
        let title_start = find_after(html_bytes, result_start, "rel=\"nofollow\">");

        let (url, title) = match (href_start, title_start) {
            (Some(hs), Some(ts)) => {
                let url = extract_attr_value(html, hs..);
                let title_text = extract_tag_content(html, ts..).unwrap_or_default();
                (url.unwrap_or_default(), title_text)
            }
            _ => {
                pos = result_start + 1;
                continue;
            }
        };

        // Find snippet: <td class="result-snippet">
        let snippet_start = find_after(html_bytes, result_start, "class=\"result-snippet\"");
        let snippet = match snippet_start {
            Some(ss) => {
                let td_start = find_after(html_bytes, ss, ">");
                match td_start {
                    Some(td) => {
                        let end = html[td + 1..].find("</td>").map(|e| td + 1 + e).unwrap_or(html.len());
                        html_to_text(&html[td + 1..end])
                    }
                    None => String::new(),
                }
            }
            None => String::new(),
        };

        if !title.is_empty() || !snippet.is_empty() {
            results.push(DdgResult {
                title: title.trim().to_string(),
                url: url.trim().to_string(),
                snippet: snippet.trim().to_string(),
            });
        }

        pos = result_start + 1;
    }

    results
}

// ---------------------------------------------------------------------------
// Minimal HTML helpers
// ---------------------------------------------------------------------------

/// Find `needle` in `haystack` as bytes after position `start`.
fn find_after(haystack: &[u8], start: usize, needle: &str) -> Option<usize> {
    let needle_bytes = needle.as_bytes();
    let slice = haystack.get(start..)?;
    let offset = slice.windows(needle_bytes.len()).position(|w| w == needle_bytes)?;
    Some(start + offset)
}

/// Extract a quoted attribute value starting after `href="`.
fn extract_attr_value(text: &str, range: std::ops::RangeFrom<usize>) -> Option<String> {
    let fragment = text.get(range)?;
    let end = fragment.find('"')?;
    Some(fragment[..end].to_string())
}

/// Extract text content inside an HTML tag starting after `>`.
fn extract_tag_content(text: &str, range: std::ops::RangeFrom<usize>) -> Option<String> {
    let fragment = text.get(range)?;
    let end = fragment.find('<')?;
    Some(fragment[..end].to_string())
}

/// Strip HTML tags from a text fragment.
fn html_to_text(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => {
                if !in_tag {
                    result.push(ch);
                }
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_fetch_rejects_invalid_urls() {
        let tool = WebFetchTool;
        let ctx = ToolContext::new(
            crate::workspace::Workspace::new("test", "Test", std::path::PathBuf::from("."), crate::workspace::ExecutorKind::LocalAndroid),
        );
        let result = tool.execute(json!({"url": "ftp://example.com/file.txt"}), &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("URL must start with http"));
    }

    #[test]
    fn web_fetch_missing_url_is_rejected() {
        let tool = WebFetchTool;
        let ctx = ToolContext::new(
            crate::workspace::Workspace::new("test", "Test", std::path::PathBuf::from("."), crate::workspace::ExecutorKind::LocalAndroid),
        );
        let result = tool.execute(json!({}), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn web_search_missing_query_is_rejected() {
        let tool = WebSearchTool;
        let ctx = ToolContext::new(
            crate::workspace::Workspace::new("test", "Test", std::path::PathBuf::from("."), crate::workspace::ExecutorKind::LocalAndroid),
        );
        let result = tool.execute(json!({}), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn tool_names_and_capabilities() {
        let fetch = WebFetchTool;
        let search = WebSearchTool;
        assert_eq!(fetch.name(), "web_fetch");
        assert_eq!(search.name(), "web_search");

        let fetch_caps = fetch.capabilities();
        assert!(fetch_caps.contains(&ToolCapability::Network));
        assert!(fetch_caps.contains(&ToolCapability::ReadOnly));

        let search_caps = search.capabilities();
        assert!(search_caps.contains(&ToolCapability::Network));
        assert!(search_caps.contains(&ToolCapability::ReadOnly));
    }

    #[test]
    fn html_to_text_strips_tags() {
        assert_eq!(html_to_text("<b>hello</b> world"), "hello world");
        assert_eq!(
            html_to_text("<div>line1<br>line2</div>"),
            "line1line2"
        );
    }

    #[test]
    fn parse_ddg_empty_html_returns_empty() {
        let results = parse_ddg_lite_html("<html></html>", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn find_after_works() {
        let bytes = b"hello world hello";
        assert_eq!(find_after(bytes, 0, "hello"), Some(0));
        assert_eq!(find_after(bytes, 1, "hello"), Some(12));
        assert_eq!(find_after(bytes, 0, "xyz"), None);
    }
}
