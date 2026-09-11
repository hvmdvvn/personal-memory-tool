//! Browser extension → raw capture (issue #12).

use crate::db::{Database, RawCapture};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BrowserCapturePayload {
    pub url: String,
    pub title: String,
    /// Exact selected text; may be empty.
    pub selection: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserCaptureError {
    Database(String),
}

impl std::fmt::Display for BrowserCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserCaptureError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for BrowserCaptureError {}

/// Persist browser URL/title/selection as an immutable raw capture.
pub fn save_browser_capture(
    db: &Database,
    payload: &BrowserCapturePayload,
) -> Result<String, BrowserCaptureError> {
    let id = Uuid::new_v4().to_string();
    let capture = RawCapture {
        id: id.clone(),
        original_content: payload.selection.clone(),
        captured_at: crate::capture::utc_now_iso8601_for_media(),
        source_kind: Some("browser".into()),
        source_app: Some("browser".into()),
        source_title: Some(payload.title.clone()),
        source_url: Some(payload.url.clone()),
        source_extra: None,
        media_path: None,
    };
    db.insert_raw_capture(&capture)
        .map_err(|e| BrowserCaptureError::Database(e.to_string()))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn saves_selection_and_browser_context_exactly() {
        let db = Database::open_in_memory().unwrap();
        let payload = BrowserCapturePayload {
            url: "https://example.com/a".into(),
            title: "Example".into(),
            selection: "  keep spaces\nand newlines  ".into(),
        };
        let id = save_browser_capture(&db, &payload).unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.original_content, payload.selection);
        assert_eq!(loaded.source_kind.as_deref(), Some("browser"));
        assert_eq!(loaded.source_app.as_deref(), Some("browser"));
        assert_eq!(loaded.source_title.as_deref(), Some("Example"));
        assert_eq!(loaded.source_url.as_deref(), Some("https://example.com/a"));
        assert_eq!(db.count_ai_metadata_for(&id).unwrap(), 0);
    }

    #[test]
    fn empty_selection_still_stores_row() {
        let db = Database::open_in_memory().unwrap();
        let payload = BrowserCapturePayload {
            url: "https://example.com/".into(),
            title: "No selection".into(),
            selection: String::new(),
        };
        let id = save_browser_capture(&db, &payload).unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.original_content, "");
        assert_eq!(loaded.source_url.as_deref(), Some("https://example.com/"));
    }
}
