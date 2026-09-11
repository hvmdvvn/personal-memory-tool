//! Clipboard → raw capture (issue #6).

use crate::db::{Database, RawCapture};
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
pub fn save_clipboard_capture(db: &Database, text: &str) -> Result<String, CaptureError> {
    if text.is_empty() {
        return Err(CaptureError::EmptyClipboard);
    }

    let id = Uuid::new_v4().to_string();
    let capture = RawCapture {
        id: id.clone(),
        original_content: text.to_string(),
        captured_at: utc_now_iso8601(),
        source_kind: Some("clipboard".into()),
        source_app: None,
        source_title: None,
        source_url: None,
        source_extra: None,
        media_path: None,
    };

    db.insert_raw_capture(&capture)
        .map_err(|e| CaptureError::Database(e.to_string()))?;
    Ok(id)
}

/// Read clipboard and save in one step (production path).
pub fn capture_clipboard_to_db(db: &Database) -> Result<String, CaptureError> {
    let text = read_clipboard_text()?;
    save_clipboard_capture(db, &text)
}

fn utc_now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal RFC3339 UTC without external clock crate dependency beyond std.
    // Format: YYYY-MM-DDTHH:MM:SSZ via a tiny breakdown.
    epoch_secs_to_iso8601(secs)
}

fn epoch_secs_to_iso8601(secs: u64) -> String {
    // Civil date from Unix day (Howard Hinnant algorithm).
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
        let id = save_clipboard_capture(&db, text).unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.original_content, text);
        assert_eq!(loaded.source_kind.as_deref(), Some("clipboard"));
        assert!(loaded.source_app.is_none());
        assert_eq!(loaded.id, id);
        assert_eq!(db.count_ai_metadata_for(&id).unwrap(), 0);
    }

    #[test]
    fn empty_clipboard_does_not_insert() {
        let db = Database::open_in_memory().unwrap();
        let err = save_clipboard_capture(&db, "").unwrap_err();
        assert_eq!(err, CaptureError::EmptyClipboard);
        assert_eq!(db.count_raw_captures().unwrap(), 0);
    }

    #[test]
    fn epoch_iso_format_smoke() {
        let s = epoch_secs_to_iso8601(0);
        assert_eq!(s, "1970-01-01T00:00:00Z");
    }
}
