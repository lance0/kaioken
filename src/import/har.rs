//! HAR (HTTP Archive) file import
//!
//! Converts HAR files exported from browser DevTools into kaioken TOML config.

use regex_lite::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct HarFile {
    log: HarLog,
}

#[derive(Debug, Deserialize)]
struct HarLog {
    entries: Vec<HarEntry>,
}

#[derive(Debug, Deserialize)]
struct HarEntry {
    request: HarRequest,
    #[serde(default)]
    #[allow(dead_code)]
    response: Option<HarResponse>,
}

#[derive(Debug, Deserialize)]
struct HarRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<HarHeader>,
    #[serde(default)]
    #[serde(rename = "postData")]
    post_data: Option<HarPostData>,
}

#[derive(Debug, Deserialize)]
struct HarHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct HarPostData {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    #[serde(rename = "mimeType")]
    #[allow(dead_code)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HarResponse {
    status: u16,
}

/// Headers to skip when importing (browser-specific or auto-generated)
const SKIP_HEADERS: &[&str] = &[
    "accept-encoding",
    "accept-language",
    "cache-control",
    "connection",
    "cookie",
    "host",
    "origin",
    "pragma",
    "referer",
    "sec-ch-ua",
    "sec-ch-ua-mobile",
    "sec-ch-ua-platform",
    "sec-fetch-dest",
    "sec-fetch-mode",
    "sec-fetch-site",
    "sec-fetch-user",
    "upgrade-insecure-requests",
    "user-agent",
    ":authority",
    ":method",
    ":path",
    ":scheme",
];

pub fn import_har(path: &Path, filter: Option<&Regex>) -> Result<String, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read HAR file: {}", e))?;

    let har: HarFile =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse HAR file: {}", e))?;

    if har.log.entries.is_empty() {
        return Err("HAR file contains no requests".to_string());
    }

    // Filter and deduplicate entries
    let entries: Vec<&HarEntry> = har
        .log
        .entries
        .iter()
        .filter(|e| {
            // Skip non-HTTP requests (data URLs, etc.)
            if !e.request.url.starts_with("http://") && !e.request.url.starts_with("https://") {
                return false;
            }
            // Apply URL filter if provided
            if let Some(re) = filter {
                return re.is_match(&e.request.url);
            }
            true
        })
        .collect();

    if entries.is_empty() {
        return Err("No matching HTTP requests found in HAR file".to_string());
    }

    // Group by URL pattern to detect if we need scenarios
    let unique_urls: std::collections::HashSet<_> = entries
        .iter()
        .map(|e| normalize_url(&e.request.url))
        .collect();

    let use_scenarios = unique_urls.len() > 1;

    // Build TOML config
    let mut config = String::new();
    config.push_str("# Generated from HAR file by kaioken import\n");
    config.push_str(&format!("# Source: {}\n", path.display()));
    config.push_str(&format!("# Entries: {} requests\n\n", entries.len()));

    if use_scenarios {
        // Multiple unique URLs - use scenarios
        config.push_str("[load]\n");
        config.push_str("concurrency = 10\n");
        config.push_str("duration = \"30s\"\n\n");

        // Count occurrences of each unique request (method + normalized URL)
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut first_entry: HashMap<String, &HarEntry> = HashMap::new();

        for entry in &entries {
            let key = format!(
                "{} {}",
                entry.request.method,
                normalize_url(&entry.request.url)
            );
            *counts.entry(key.clone()).or_insert(0) += 1;
            first_entry.entry(key).or_insert(entry);
        }

        // Sort entries for deterministic output order
        let mut sorted_entries: Vec<_> = first_entry.into_iter().collect();
        sorted_entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        // Generate scenarios with weights based on occurrence count
        for (key, entry) in sorted_entries {
            let weight = counts.get(&key).copied().unwrap_or(1);

            config.push_str("[[scenarios]]\n");
            config.push_str(&format!("url = \"{}\"\n", escape_toml(&entry.request.url)));
            config.push_str(&format!("method = \"{}\"\n", entry.request.method));
            if weight > 1 {
                config.push_str(&format!("weight = {}\n", weight));
            }

            // Add body BEFORE headers (must be in [[scenarios]] table, not [scenarios.headers])
            if let Some(ref post_data) = entry.request.post_data
                && let Some(ref text) = post_data.text
                && !text.is_empty()
            {
                config.push_str(&format_body(text));
            }

            // Add headers AFTER body
            let headers = filter_headers(&entry.request.headers);
            if !headers.is_empty() {
                config.push_str("[scenarios.headers]\n");
                for (name, value) in headers {
                    config.push_str(&format!(
                        "{} = \"{}\"\n",
                        format_header_key(&name),
                        escape_toml(&value)
                    ));
                }
            }

            config.push('\n');
        }
    } else {
        // Single URL pattern - use simple config
        let entry = entries.first().unwrap();

        config.push_str("[target]\n");
        config.push_str(&format!("url = \"{}\"\n", escape_toml(&entry.request.url)));
        config.push_str(&format!("method = \"{}\"\n", entry.request.method));

        // Add body BEFORE headers (must be in [target] table, not [target.headers])
        if let Some(ref post_data) = entry.request.post_data
            && let Some(ref text) = post_data.text
            && !text.is_empty()
        {
            config.push_str(&format_body(text));
        }

        // Add headers AFTER body
        let headers = filter_headers(&entry.request.headers);
        if !headers.is_empty() {
            config.push_str("\n[target.headers]\n");
            for (name, value) in headers {
                config.push_str(&format!(
                    "{} = \"{}\"\n",
                    format_header_key(&name),
                    escape_toml(&value)
                ));
            }
        }

        config.push_str("\n[load]\n");
        config.push_str("concurrency = 10\n");
        config.push_str("duration = \"30s\"\n");
    }

    // Add suggested thresholds
    config.push_str("\n# Suggested thresholds (adjust based on your SLOs)\n");
    config.push_str("# [thresholds]\n");
    config.push_str("# p99_latency_ms = \"< 500\"\n");
    config.push_str("# error_rate = \"< 0.01\"\n");

    Ok(config)
}

/// Filter out browser-specific headers
fn filter_headers(headers: &[HarHeader]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|h| {
            let name_lower = h.name.to_lowercase();
            !SKIP_HEADERS.contains(&name_lower.as_str())
        })
        .map(|h| (h.name.clone(), h.value.clone()))
        .collect()
}

/// Normalize URL for deduplication (remove query params with dynamic values)
fn normalize_url(url: &str) -> String {
    // Simple normalization: keep scheme + host + path, ignore query
    if let Some(idx) = url.find('?') {
        url[..idx].to_string()
    } else {
        url.to_string()
    }
}

/// Escape special characters for TOML strings
fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Quote header name for TOML key (escape special chars)
fn format_header_key(name: &str) -> String {
    // TOML keys with special characters need to be quoted and escaped
    format!("\"{}\"", escape_toml(name))
}

/// Format body for TOML output, handling payloads that contain '''
fn format_body(text: &str) -> String {
    if text.contains("'''") {
        // Fall back to basic string with escaping
        format!("body = \"{}\"\n", escape_toml(text))
    } else {
        // Use literal string (preserves content as-is)
        format!("body = '''\n{}'''\n", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://api.example.com/users?id=123"),
            "https://api.example.com/users"
        );
        assert_eq!(
            normalize_url("https://api.example.com/users"),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn test_escape_toml() {
        assert_eq!(escape_toml("hello\"world"), "hello\\\"world");
        assert_eq!(escape_toml("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_filter_headers_removes_browser_headers() {
        let headers = vec![
            HarHeader {
                name: "Accept".to_string(),
                value: "application/json".to_string(),
            },
            HarHeader {
                name: "User-Agent".to_string(),
                value: "Mozilla/5.0".to_string(),
            },
            HarHeader {
                name: "Cache-Control".to_string(),
                value: "no-cache".to_string(),
            },
            HarHeader {
                name: "Authorization".to_string(),
                value: "Bearer token".to_string(),
            },
            HarHeader {
                name: "sec-fetch-mode".to_string(),
                value: "cors".to_string(),
            },
        ];
        let filtered = filter_headers(&headers);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|(n, _)| n == "Accept"));
        assert!(filtered.iter().any(|(n, _)| n == "Authorization"));
    }

    #[test]
    fn test_format_header_key() {
        assert_eq!(format_header_key("Content-Type"), "\"Content-Type\"");
        assert_eq!(format_header_key("X-Custom"), "\"X-Custom\"");
    }

    #[test]
    fn test_format_header_key_escapes_quotes() {
        // Header names with quotes should be escaped
        assert_eq!(format_header_key("X-\"Test\""), "\"X-\\\"Test\\\"\"");
        assert_eq!(format_header_key("X-Back\\slash"), "\"X-Back\\\\slash\"");
    }

    #[test]
    fn test_format_body_with_triple_quotes() {
        // Body containing ''' should use escaped basic string
        let body_with_quotes = "some text with ''' in it";
        let result = format_body(body_with_quotes);
        assert!(result.starts_with("body = \""));
        assert!(result.contains("'''"));
        assert!(!result.contains("body = '''"));
    }

    #[test]
    fn test_format_body_normal() {
        // Normal body should use literal string
        let normal_body = "{\"key\": \"value\"}";
        let result = format_body(normal_body);
        assert!(result.starts_with("body = '''"));
    }

    #[test]
    fn test_import_har_single_url() {
        let har = r#"{
            "log": {
                "entries": [{
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/health",
                        "headers": []
                    }
                }]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let result = import_har(file.path(), None).unwrap();
        assert!(result.contains("[target]"));
        assert!(result.contains("url = \"https://api.example.com/health\""));
        assert!(result.contains("method = \"GET\""));
        assert!(result.contains("[load]"));
    }

    #[test]
    fn test_import_har_multiple_urls_creates_scenarios() {
        let har = r#"{
            "log": {
                "entries": [
                    {"request": {"method": "GET", "url": "https://api.example.com/users", "headers": []}},
                    {"request": {"method": "GET", "url": "https://api.example.com/posts", "headers": []}}
                ]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let result = import_har(file.path(), None).unwrap();
        assert!(result.contains("[[scenarios]]"));
        assert!(result.contains("/users"));
        assert!(result.contains("/posts"));
    }

    #[test]
    fn test_import_har_body_before_headers() {
        let har = r#"{
            "log": {
                "entries": [{
                    "request": {
                        "method": "POST",
                        "url": "https://api.example.com/data",
                        "headers": [{"name": "Content-Type", "value": "application/json"}],
                        "postData": {"text": "{\"key\": \"value\"}"}
                    }
                }]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let result = import_har(file.path(), None).unwrap();

        // Body must appear BEFORE [target.headers] to be valid TOML
        let body_pos = result.find("body = '''").unwrap();
        let headers_pos = result.find("[target.headers]").unwrap();
        assert!(
            body_pos < headers_pos,
            "body must come before headers section"
        );
    }

    #[test]
    fn test_import_har_with_filter() {
        let har = r#"{
            "log": {
                "entries": [
                    {"request": {"method": "GET", "url": "https://api.example.com/v1/users", "headers": []}},
                    {"request": {"method": "GET", "url": "https://api.example.com/v2/users", "headers": []}},
                    {"request": {"method": "GET", "url": "https://cdn.example.com/image.png", "headers": []}}
                ]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let filter = Regex::new("api.example.com/v2").unwrap();
        let result = import_har(file.path(), Some(&filter)).unwrap();

        assert!(result.contains("/v2/users"));
        assert!(!result.contains("/v1/users"));
        assert!(!result.contains("cdn.example.com"));
    }

    #[test]
    fn test_import_har_empty_file_fails() {
        let har = r#"{"log": {"entries": []}}"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let result = import_har(file.path(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no requests"));
    }

    #[test]
    fn test_import_har_skips_data_urls() {
        let har = r#"{
            "log": {
                "entries": [
                    {"request": {"method": "GET", "url": "data:text/plain;base64,SGVsbG8=", "headers": []}},
                    {"request": {"method": "GET", "url": "https://api.example.com/health", "headers": []}}
                ]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let result = import_har(file.path(), None).unwrap();
        assert!(!result.contains("data:text"));
        assert!(result.contains("api.example.com"));
    }

    #[test]
    fn test_import_har_weights_from_duplicates() {
        let har = r#"{
            "log": {
                "entries": [
                    {"request": {"method": "GET", "url": "https://api.example.com/health", "headers": []}},
                    {"request": {"method": "GET", "url": "https://api.example.com/health", "headers": []}},
                    {"request": {"method": "GET", "url": "https://api.example.com/health", "headers": []}},
                    {"request": {"method": "POST", "url": "https://api.example.com/data", "headers": []}}
                ]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(har.as_bytes()).unwrap();

        let result = import_har(file.path(), None).unwrap();
        assert!(
            result.contains("weight = 3"),
            "GET /health should have weight 3"
        );
    }
}
