//! Local app config (issue #28) + loopback guardrails (issue #30).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::classify::DEFAULT_CLASSIFY_MODEL;
use crate::embeddings::DEFAULT_EMBED_MODEL;
use crate::ollama::DEFAULT_OLLAMA_BASE_URL;
use crate::shortcut::DEFAULT_GLOBAL_SHORTCUT;

pub const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub ollama_base_url: String,
    pub chat_model: String,
    pub embed_model: String,
    /// Documented hotkey string; live rebind may require restart (#28).
    pub global_hotkey: String,
    /// Optional override for data directory; `None` → Tauri app data dir.
    pub data_dir: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ollama_base_url: DEFAULT_OLLAMA_BASE_URL.to_string(),
            chat_model: DEFAULT_CLASSIFY_MODEL.to_string(),
            embed_model: DEFAULT_EMBED_MODEL.to_string(),
            global_hotkey: DEFAULT_GLOBAL_SHORTCUT.to_string(),
            data_dir: None,
        }
    }
}

impl AppConfig {
    pub fn config_path(data_dir: &Path) -> PathBuf {
        data_dir.join(CONFIG_FILE_NAME)
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("read config: {e}"))?;
        let mut cfg: AppConfig =
            serde_json::from_str(&raw).map_err(|e| format!("parse config: {e}"))?;
        // Fill blanks with defaults if someone wrote partial JSON via merge-friendly deserialize:
        if cfg.ollama_base_url.trim().is_empty() {
            cfg.ollama_base_url = DEFAULT_OLLAMA_BASE_URL.to_string();
        }
        if cfg.chat_model.trim().is_empty() {
            cfg.chat_model = DEFAULT_CLASSIFY_MODEL.to_string();
        }
        if cfg.embed_model.trim().is_empty() {
            cfg.embed_model = DEFAULT_EMBED_MODEL.to_string();
        }
        if cfg.global_hotkey.trim().is_empty() {
            cfg.global_hotkey = DEFAULT_GLOBAL_SHORTCUT.to_string();
        }
        Ok(cfg)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create config dir: {e}"))?;
        }
        let raw = serde_json::to_string_pretty(self).map_err(|e| format!("serialize config: {e}"))?;
        fs::write(path, raw).map_err(|e| format!("write config: {e}"))
    }

    pub fn load_from_data_dir(data_dir: &Path) -> Result<Self, String> {
        Self::load_from_path(&Self::config_path(data_dir))
    }

    pub fn save_to_data_dir(&self, data_dir: &Path) -> Result<(), String> {
        self.save_to_path(&Self::config_path(data_dir))
    }
}

/// True if host is loopback (`127.0.0.1`, `localhost`, `::1`) after normalizing the base URL.
pub fn is_loopback_base_url(raw: &str) -> bool {
    let base = crate::ollama::normalize_base_url(raw);
    let without_scheme = base
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let authority = without_scheme.split('/').next().unwrap_or("");
    let host = if let Some(inner) = authority.strip_prefix('[') {
        // IPv6 literal: [::1] or [::1]:11434
        inner.split(']').next().unwrap_or("").to_ascii_lowercase()
    } else {
        authority
            .rsplit_once(':')
            .map(|(h, _)| h)
            .unwrap_or(authority)
            .to_ascii_lowercase()
    };
    matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1")
}

/// Log a warning when AI traffic would leave the machine (issue #30). Cloud providers are out of scope for v1.
pub fn warn_if_non_loopback(cfg: &AppConfig) {
    if !is_loopback_base_url(&cfg.ollama_base_url) {
        eprintln!(
            "[privacy] WARNING: ollama_base_url is not loopback ({}); \
             v1 expects local-only Ollama. Cloud LLM providers are out of scope.",
            cfg.ollama_base_url
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn defaults_are_local() {
        let cfg = AppConfig::default();
        assert!(is_loopback_base_url(&cfg.ollama_base_url));
        assert_eq!(cfg.chat_model, "llama3.2");
        assert_eq!(cfg.embed_model, "nomic-embed-text");
        assert!(!cfg.global_hotkey.is_empty());
        assert!(cfg.data_dir.is_none());
    }

    #[test]
    fn load_override_from_temp_file() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pmt-config-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        let path = AppConfig::config_path(&dir);
        let mut cfg = AppConfig::default();
        cfg.chat_model = "custom-chat".into();
        cfg.ollama_base_url = "http://10.0.0.5:11434".into();
        cfg.save_to_path(&path).unwrap();

        let loaded = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(loaded.chat_model, "custom-chat");
        assert!(!is_loopback_base_url(&loaded.ollama_base_url));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_yields_defaults() {
        let path = std::env::temp_dir().join("pmt-config-missing-does-not-exist.json");
        let _ = fs::remove_file(&path);
        let cfg = AppConfig::load_from_path(&path).unwrap();
        assert_eq!(cfg, AppConfig::default());
    }

    #[test]
    fn loopback_detection() {
        assert!(is_loopback_base_url("http://127.0.0.1:11434"));
        assert!(is_loopback_base_url("localhost:11434"));
        assert!(is_loopback_base_url("http://[::1]:11434"));
        assert!(!is_loopback_base_url("http://example.com:11434"));
        assert!(!is_loopback_base_url("http://192.168.1.10:11434"));
    }
}
