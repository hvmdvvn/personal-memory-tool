//! Capture orchestration for the global shortcut (issue #9).

use crate::capture::{save_clipboard_capture, CaptureError};
use crate::db::{Database, RawCapture};
use crate::media::{media_dir, save_screenshot_capture, MediaError};
use crate::window_context::WindowContext;
use std::path::Path;
use uuid::Uuid;

/// Default: take a screenshot when clipboard is empty.
pub const DEFAULT_SCREENSHOT_ON_EMPTY_CLIPBOARD: bool = true;

#[derive(Debug, Clone)]
pub struct CaptureNowOptions {
    pub screenshot_on_empty_clipboard: bool,
}

impl Default for CaptureNowOptions {
    fn default() -> Self {
        Self {
            screenshot_on_empty_clipboard: DEFAULT_SCREENSHOT_ON_EMPTY_CLIPBOARD,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardInput {
    Text(String),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestrateError {
    Capture(String),
    Media(String),
    Database(String),
}

impl std::fmt::Display for OrchestrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestrateError::Capture(e) => write!(f, "{e}"),
            OrchestrateError::Media(e) => write!(f, "{e}"),
            OrchestrateError::Database(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OrchestrateError {}

impl From<CaptureError> for OrchestrateError {
    fn from(value: CaptureError) -> Self {
        OrchestrateError::Capture(value.to_string())
    }
}

impl From<MediaError> for OrchestrateError {
    fn from(value: MediaError) -> Self {
        OrchestrateError::Media(value.to_string())
    }
}

/// Testable orchestration: clipboard wins; otherwise fallback (± screenshot bytes).
pub fn capture_now_with(
    db: &Database,
    app_data_dir: &Path,
    clipboard: ClipboardInput,
    window: &WindowContext,
    screenshot_png: Option<&[u8]>,
    opts: &CaptureNowOptions,
) -> Result<String, OrchestrateError> {
    let window_ref = if window.is_empty() {
        None
    } else {
        Some(window)
    };

    match clipboard {
        ClipboardInput::Text(text) if !text.is_empty() => {
            Ok(save_clipboard_capture(db, &text, window_ref)?)
        }
        ClipboardInput::Text(_) | ClipboardInput::Empty => {
            if opts.screenshot_on_empty_clipboard {
                let png = screenshot_png.ok_or_else(|| {
                    OrchestrateError::Media(
                        "screenshot fallback enabled but no PNG bytes provided".into(),
                    )
                })?;
                Ok(save_screenshot_capture(
                    db,
                    &media_dir(app_data_dir),
                    png,
                    window_ref,
                    "",
                )?)
            } else {
                save_context_only_capture(db, window_ref)
            }
        }
    }
}

fn save_context_only_capture(
    db: &Database,
    window: Option<&WindowContext>,
) -> Result<String, OrchestrateError> {
    let id = Uuid::new_v4().to_string();
    let capture = RawCapture {
        id: id.clone(),
        original_content: String::new(),
        captured_at: crate::capture::utc_now_iso8601_for_media(),
        source_kind: Some("context".into()),
        source_app: window.and_then(|w| w.app.clone()),
        source_title: window.and_then(|w| w.title.clone()),
        source_url: None,
        source_extra: None,
        media_path: None,
    };
    db.insert_raw_capture(&capture)
        .map_err(|e| OrchestrateError::Database(e.to_string()))?;
    Ok(id)
}

/// Production path: read clipboard + optional live screenshot grab.
pub fn capture_now(
    db: &Database,
    app_data_dir: &Path,
    opts: &CaptureNowOptions,
) -> Result<String, OrchestrateError> {
    let clipboard = match crate::capture::read_clipboard_text() {
        Ok(text) => ClipboardInput::Text(text),
        Err(CaptureError::EmptyClipboard) => ClipboardInput::Empty,
        Err(CaptureError::Clipboard(_)) => ClipboardInput::Empty,
        Err(e) => return Err(e.into()),
    };

    let window = crate::window_context::foreground_window_context();

    let png_owned: Option<Vec<u8>> = if matches!(clipboard, ClipboardInput::Empty)
        && opts.screenshot_on_empty_clipboard
    {
        Some(crate::media::grab_primary_monitor_png()?)
    } else {
        None
    };

    capture_now_with(
        db,
        app_data_dir,
        clipboard,
        &window,
        png_owned.as_deref(),
        opts,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::TINY_PNG_1X1;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pm_orch_{nanos}"))
    }

    #[test]
    fn clipboard_wins_over_fallback() {
        let root = temp_root();
        let db = Database::open_in_memory().unwrap();
        let window = WindowContext {
            app: Some("App".into()),
            title: Some("Title".into()),
        };
        let opts = CaptureNowOptions {
            screenshot_on_empty_clipboard: true,
        };
        let id = capture_now_with(
            &db,
            &root,
            ClipboardInput::Text("from clipboard".into()),
            &window,
            Some(TINY_PNG_1X1),
            &opts,
        )
        .unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.source_kind.as_deref(), Some("clipboard"));
        assert_eq!(loaded.original_content, "from clipboard");
        assert!(loaded.media_path.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_clipboard_with_screenshot_flag_uses_media() {
        let root = temp_root();
        let db = Database::open_in_memory().unwrap();
        let window = WindowContext {
            app: Some("App".into()),
            title: None,
        };
        let opts = CaptureNowOptions {
            screenshot_on_empty_clipboard: true,
        };
        let id = capture_now_with(
            &db,
            &root,
            ClipboardInput::Empty,
            &window,
            Some(TINY_PNG_1X1),
            &opts,
        )
        .unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.source_kind.as_deref(), Some("screenshot"));
        assert!(loaded.media_path.is_some());
        assert_eq!(loaded.source_app.as_deref(), Some("App"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_clipboard_without_screenshot_writes_context_row() {
        let root = temp_root();
        let db = Database::open_in_memory().unwrap();
        let window = WindowContext {
            app: Some("OnlyApp".into()),
            title: Some("OnlyTitle".into()),
        };
        let opts = CaptureNowOptions {
            screenshot_on_empty_clipboard: false,
        };
        let id = capture_now_with(
            &db,
            &root,
            ClipboardInput::Empty,
            &window,
            None,
            &opts,
        )
        .unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.source_kind.as_deref(), Some("context"));
        assert_eq!(loaded.source_app.as_deref(), Some("OnlyApp"));
        assert_eq!(loaded.source_title.as_deref(), Some("OnlyTitle"));
        assert!(loaded.media_path.is_none());
        assert_eq!(db.count_raw_captures().unwrap(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
