//! Local Ollama connectivity health check (issue #13).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default Ollama HTTP base (loopback).
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

/// Env var that may override the base URL (with or without `http://` scheme).
pub const OLLAMA_HOST_ENV: &str = "OLLAMA_HOST";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaHealth {
    pub ok: bool,
    pub base_url: String,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Option<Vec<TagModel>>,
}

#[derive(Debug, Deserialize)]
struct TagModel {
    name: Option<String>,
    model: Option<String>,
}

/// Resolve base URL from env or default; always returns an `http://` URL without trailing slash.
pub fn ollama_base_url() -> String {
    normalize_base_url(
        std::env::var(OLLAMA_HOST_ENV)
            .ok()
            .as_deref()
            .unwrap_or(DEFAULT_OLLAMA_BASE_URL),
    )
}

pub fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

/// GET `{base}/api/tags` and parse model names.
pub fn check_ollama_health(base_url: &str) -> OllamaHealth {
    let base = normalize_base_url(base_url);
    let url = format!("{base}/api/tags");

    match ureq::get(&url).timeout(std::time::Duration::from_secs(3)).call() {
        Ok(resp) => match resp.into_string() {
            Ok(body) => match parse_tags_body(&body) {
                Ok(models) => OllamaHealth {
                    ok: true,
                    base_url: base,
                    models,
                    error: None,
                },
                Err(e) => OllamaHealth {
                    ok: false,
                    base_url: base,
                    models: vec![],
                    error: Some(e),
                },
            },
            Err(e) => OllamaHealth {
                ok: false,
                base_url: base,
                models: vec![],
                error: Some(format!("read body: {e}")),
            },
        },
        Err(e) => OllamaHealth {
            ok: false,
            base_url: base,
            models: vec![],
            error: Some(e.to_string()),
        },
    }
}

fn parse_tags_body(body: &str) -> Result<Vec<String>, String> {
    let parsed: TagsResponse =
        serde_json::from_str(body).map_err(|e| format!("invalid tags json: {e}"))?;
    let mut names = Vec::new();
    for m in parsed.models.unwrap_or_default() {
        if let Some(name) = m.name.or(m.model) {
            names.push(name);
        }
    }
    // Also accept alternate shapes via Value for resilience in tests.
    if names.is_empty() {
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            if let Some(arr) = v.get("models").and_then(|m| m.as_array()) {
                for item in arr {
                    if let Some(n) = item.get("name").and_then(|x| x.as_str()) {
                        names.push(n.to_string());
                    }
                }
            }
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::thread;
    use std::time::Duration;

    static PORT: AtomicU16 = AtomicU16::new(18000);

    fn next_port() -> u16 {
        PORT.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn normalize_adds_scheme_when_missing() {
        assert_eq!(
            normalize_base_url("127.0.0.1:11434"),
            "http://127.0.0.1:11434"
        );
        assert_eq!(
            normalize_base_url("http://127.0.0.1:11434/"),
            "http://127.0.0.1:11434"
        );
    }

    #[test]
    fn health_ok_against_mock_tags_server() {
        let port = next_port();
        let server =
            tiny_http::Server::http(format!("127.0.0.1:{port}")).expect("bind mock ollama");
        thread::spawn(move || {
            if let Ok(mut req) = server.recv() {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let resp = tiny_http::Response::from_string(
                    r#"{"models":[{"name":"llama3.2:latest"},{"name":"nomic-embed-text"}]}"#,
                )
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                );
                let _ = req.respond(resp);
            }
        });
        thread::sleep(Duration::from_millis(30));

        let health = check_ollama_health(&format!("http://127.0.0.1:{port}"));
        assert!(health.ok, "{health:?}");
        assert!(health.models.iter().any(|m| m.contains("llama3.2")));
        assert!(health.error.is_none());
    }

    #[test]
    fn health_fails_when_server_down() {
        let health = check_ollama_health("http://127.0.0.1:1");
        assert!(!health.ok);
        assert!(health.error.is_some());
        assert!(health.models.is_empty());
    }
}
