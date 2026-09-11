//! Screenshot / media file storage for raw captures (issue #8).

use crate::db::{Database, RawCapture};
use crate::window_context::{foreground_window_context, WindowContext};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    Io(String),
    Database(String),
    Grab(String),
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaError::Io(e) => write!(f, "media io error: {e}"),
            MediaError::Database(e) => write!(f, "database error: {e}"),
            MediaError::Grab(e) => write!(f, "screenshot grab error: {e}"),
        }
    }
}

impl std::error::Error for MediaError {}

/// Directory for screenshot PNGs under the app data root.
pub fn media_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("media")
}

/// Write PNG bytes to `{media_dir}/{id}.png` and return that path.
pub fn store_png_bytes(media_dir: &Path, id: &str, png_bytes: &[u8]) -> Result<PathBuf, MediaError> {
    fs::create_dir_all(media_dir).map_err(|e| MediaError::Io(e.to_string()))?;
    let path = media_dir.join(format!("{id}.png"));
    fs::write(&path, png_bytes).map_err(|e| MediaError::Io(e.to_string()))?;
    Ok(path)
}

/// Create a raw capture referencing an on-disk PNG. Does not write AI metadata.
/// If DB insert fails after the file was written, the file is left in place (no auto-cleanup).
pub fn save_screenshot_capture(
    db: &Database,
    media_directory: &Path,
    png_bytes: &[u8],
    window: Option<&WindowContext>,
    original_content: &str,
) -> Result<String, MediaError> {
    let id = Uuid::new_v4().to_string();
    let path = store_png_bytes(media_directory, &id, png_bytes)?;

    let capture = RawCapture {
        id: id.clone(),
        original_content: original_content.to_string(),
        captured_at: crate::capture::utc_now_iso8601_for_media(),
        source_kind: Some("screenshot".into()),
        source_app: window.and_then(|w| w.app.clone()),
        source_title: window.and_then(|w| w.title.clone()),
        source_url: None,
        source_extra: None,
        media_path: Some(path.to_string_lossy().into_owned()),
    };

    if let Err(e) = db.insert_raw_capture(&capture) {
        let _ = fs::remove_file(&path);
        return Err(MediaError::Database(e.to_string()));
    }
    Ok(id)
}

/// Grab primary monitor as PNG bytes (desktop). Soft errors → MediaError::Grab.
pub fn grab_primary_monitor_png() -> Result<Vec<u8>, MediaError> {
    #[cfg(desktop)]
    {
        use image::ImageEncoder;
        let monitors = xcap::Monitor::all().map_err(|e| MediaError::Grab(e.to_string()))?;
        let monitor = monitors
            .into_iter()
            .next()
            .ok_or_else(|| MediaError::Grab("no monitors found".into()))?;
        let rgba = monitor
            .capture_image()
            .map_err(|e| MediaError::Grab(e.to_string()))?;
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| MediaError::Grab(e.to_string()))?;
        Ok(png)
    }
    #[cfg(not(desktop))]
    {
        Err(MediaError::Grab(
            "screenshot grab not supported on this target".into(),
        ))
    }
}

/// Production path: grab screen, store PNG, insert raw capture.
pub fn capture_screenshot_to_db(db: &Database, app_data_dir: &Path) -> Result<String, MediaError> {
    let png = grab_primary_monitor_png()?;
    let ctx = foreground_window_context();
    let ctx_ref = if ctx.is_empty() { None } else { Some(&ctx) };
    save_screenshot_capture(db, &media_dir(app_data_dir), &png, ctx_ref, "")
}

/// Minimal valid 1×1 PNG (for tests).
pub const TINY_PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_media_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pm_media_test_{nanos}"))
    }

    #[test]
    fn store_png_bytes_writes_file() {
        let root = temp_media_root();
        let dir = media_dir(&root);
        let path = store_png_bytes(&dir, "fixture-id", TINY_PNG_1X1).unwrap();
        assert!(path.ends_with("fixture-id.png"));
        assert_eq!(fs::read(&path).unwrap(), TINY_PNG_1X1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn save_screenshot_capture_with_fixture() {
        let root = temp_media_root();
        let dir = media_dir(&root);
        let db = Database::open_in_memory().unwrap();
        let ctx = WindowContext {
            app: Some("TestApp".into()),
            title: Some("Title".into()),
        };
        let id = save_screenshot_capture(&db, &dir, TINY_PNG_1X1, Some(&ctx), "").unwrap();
        let loaded = db.get_raw_capture(&id).unwrap().unwrap();
        assert_eq!(loaded.source_kind.as_deref(), Some("screenshot"));
        assert_eq!(loaded.original_content, "");
        assert_eq!(loaded.source_app.as_deref(), Some("TestApp"));
        let media = loaded.media_path.expect("media_path");
        assert!(Path::new(&media).exists());
        assert_eq!(fs::read(&media).unwrap(), TINY_PNG_1X1);
        assert_eq!(db.count_ai_metadata_for(&id).unwrap(), 0);
        let _ = fs::remove_dir_all(&root);
    }
}
