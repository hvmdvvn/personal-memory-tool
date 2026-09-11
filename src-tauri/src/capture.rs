//! Clipboard → raw capture (issues #6–#7).

use crate::db::{Database, RawCapture};
use crate::window_context::{foreground_window_context, WindowContext};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    EmptyClipboard,
    Database(String),
    Clipboard(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::EmptyClipboard => write!(f, "clipboard is empty or has no text"),
            CaptureError::Database(e) => write!(f, "database error: {e}"),
            CaptureError::Clipboard(e) => write!(f, "clipboard error: {e}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Read plain text from the system clipboard.
pub fn read_clipboard_text() -> Result<String, CaptureError> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| CaptureError::Clipboard(e.to_string()))?;
    match clipboard.get_text() {
        Ok(text) if !text.is_empty() => Ok(text),
        Ok(_) => Err(CaptureError::EmptyClipboard),
        Err(arboard::Error::ContentNotAvailable) => Err(CaptureError::EmptyClipboard),
        Err(e) => Err(CaptureError::Clipboard(e.to_string())),
    }
}

/// Persist clipboard text as an immutable raw capture. Returns the new id.
pub fn save_clipboard_capture(
    db: &Database,
    text: &str,
    window: Option<&WindowContext>,
) -> Result<String, CaptureError> {
    if text.is_empty() {
        return Err(CaptureError::EmptyClipboard);
    }

    let id = Uuid::new_v4().to_string();
    let capture = RawCapture {
        id: id.clone(),
        original_content: text.to_string(),
        captured_at: utc_now_iso8601(),
        source_kind: Some("clipboard".into()),
        source_app: window.and_then(|w| w.app.clone()),
        source_title: window.and_then(|w| w.title.clone()),
        source_url: None,
        source_extra: None,
        media_path: None,
    };

    db.insert_raw_capture(&capture)
        .map_err(|e| CaptureError::Database(e.to_string()))?;
    Ok(id)
}

/// Read clipboard + soft-fail window context, then save.
pub fn capture_clipboard_to_db(db: &Database) -> Result<String, CaptureError> {
    let text = read_clipboard_text()?;
    let ctx = foreground_window_context();
    let ctx_ref = if ctx.is_empty() { None } else { Some(&ctx) };
    save_clipboard_capture(db, &text, ctx_ref)
}

fn utc_now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    epoch_secs_to_iso8601(secs)
}

/// Shared UTC timestamp helper for other capture kinds (e.g. screenshots).
pub(crate) fn utc_now_iso8601_for_media() -> String {
    utc_now_iso8601()
}

fn epoch_secs_to_iso8601(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn save_mocked_clipboard_text_round_trips() {
        let db = Database::open_in_memory().unwrap();
        let text = "  keep exact clipboard\ntext  ";
        let id = save_clipboard_capture(&db, text, None).unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.original_content, text);
        assert_eq!(loaded.source_kind.as_deref(), Some("clipboard"));
        assert!(loaded.source_app.is_none());
        assert_eq!(loaded.id, id);
        assert_eq!(db.count_ai_metadata_for(&id).unwrap(), 0);
    }

    #[test]
    fn save_applies_window_context_when_provided() {
        let db = Database::open_in_memory().unwrap();
        let ctx = WindowContext {
            app: Some("Code".into()),
            title: Some("plan.md - spec".into()),
        };
        let id = save_clipboard_capture(&db, "hello", Some(&ctx)).unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.source_app.as_deref(), Some("Code"));
        assert_eq!(loaded.source_title.as_deref(), Some("plan.md - spec"));
        assert_eq!(loaded.original_content, "hello");
    }

    #[test]
    fn missing_window_context_still_saves() {
        let db = Database::open_in_memory().unwrap();
        let id = save_clipboard_capture(&db, "no-window", None).unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert!(loaded.source_app.is_none());
        assert!(loaded.source_title.is_none());
    }

    #[test]
    fn empty_clipboard_does_not_insert() {
        let db = Database::open_in_memory().unwrap();
        let err = save_clipboard_capture(&db, "", None).unwrap_err();
        assert_eq!(err, CaptureError::EmptyClipboard);
        assert_eq!(db.count_raw_captures().unwrap(), 0);
    }

    #[test]
    fn epoch_iso_format_smoke() {
        let s = epoch_secs_to_iso8601(0);
        assert_eq!(s, "1970-01-01T00:00:00Z");
    }
}
