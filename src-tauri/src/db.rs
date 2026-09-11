//! Local SQLite persistence for raw captures (issue #3).
//!
//! Applies migration `v001_captures` from `_docs/data-model/schema_v001.sql`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Migration identity recorded in `schema_migrations`.
pub const MIGRATION_V001_CAPTURES: &str = "v001_captures";

const V001_SQL: &str = include_str!("../../_docs/data-model/schema_v001.sql");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCapture {
    pub id: String,
    pub original_content: String,
    pub captured_at: String,
    pub source_kind: Option<String>,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
    pub source_url: Option<String>,
    pub source_extra: Option<String>,
    pub media_path: Option<String>,
}

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a SQLite database at `path` and apply pending migrations.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.configure()?;
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.configure()?;
        db.migrate()?;
        Ok(db)
    }

    fn configure(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(())
    }

    /// Apply recorded migrations. Safe to call multiple times.
    pub fn migrate(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                id TEXT PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )?;

        if !self.migration_applied(MIGRATION_V001_CAPTURES)? {
            self.conn.execute_batch(V001_SQL)?;
            self.conn.execute(
                "INSERT INTO schema_migrations (id, applied_at) VALUES (?1, datetime('now'));",
                params![MIGRATION_V001_CAPTURES],
            )?;
        }

        // Ensure FKs remain on after any SQL that may have set them.
        self.configure()?;
        Ok(())
    }

    fn migration_applied(&self, id: &str) -> rusqlite::Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM schema_migrations WHERE id = ?1")?;
        let found = stmt.exists(params![id])?;
        Ok(found)
    }

    pub fn insert_raw_capture(&self, capture: &RawCapture) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO captures (
                id, original_content, captured_at,
                source_kind, source_app, source_title, source_url, source_extra, media_path
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                capture.id,
                capture.original_content,
                capture.captured_at,
                capture.source_kind,
                capture.source_app,
                capture.source_title,
                capture.source_url,
                capture.source_extra,
                capture.media_path,
            ],
        )?;
        Ok(())
    }

    pub fn get_raw_capture(&self, id: &str) -> rusqlite::Result<Option<RawCapture>> {
        self.conn
            .query_row(
                "SELECT id, original_content, captured_at,
                        source_kind, source_app, source_title, source_url, source_extra, media_path
                 FROM captures WHERE id = ?1",
                params![id],
                |row| {
                    Ok(RawCapture {
                        id: row.get(0)?,
                        original_content: row.get(1)?,
                        captured_at: row.get(2)?,
                        source_kind: row.get(3)?,
                        source_app: row.get(4)?,
                        source_title: row.get(5)?,
                        source_url: row.get(6)?,
                        source_extra: row.get(7)?,
                        media_path: row.get(8)?,
                    })
                },
            )
            .optional()
    }

    /// Test helper: whether foreign_keys pragma is on.
    pub fn foreign_keys_enabled(&self) -> rusqlite::Result<bool> {
        let v: i64 = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        Ok(v == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("personal_memory_test_{nanos}.sqlite"))
    }

    fn sample_capture() -> RawCapture {
        RawCapture {
            id: "cap-1".into(),
            original_content: "  exact quote\nwith newlines  ".into(),
            captured_at: "2026-09-11T15:00:00Z".into(),
            source_kind: Some("clipboard".into()),
            source_app: Some("TestApp".into()),
            source_title: Some("Window".into()),
            source_url: None,
            source_extra: None,
            media_path: None,
        }
    }

    #[test]
    fn open_migrate_insert_round_trip_temp_file() {
        let path = temp_db_path();
        let _ = std::fs::remove_file(&path);

        let db = Database::open(&path).expect("open");
        let capture = sample_capture();
        db.insert_raw_capture(&capture).expect("insert");
        let loaded = db
            .get_raw_capture("cap-1")
            .expect("query")
            .expect("found");
        assert_eq!(loaded, capture);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn migrate_is_idempotent() {
        let db = Database::open_in_memory().expect("open");
        db.migrate().expect("second migrate");
        db.migrate().expect("third migrate");
        assert!(db.migration_applied(MIGRATION_V001_CAPTURES).unwrap());
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let db = Database::open_in_memory().expect("open");
        assert!(db.foreign_keys_enabled().unwrap());
    }

    #[test]
    fn insert_without_ai_metadata_and_null_context() {
        let db = Database::open_in_memory().expect("open");
        let capture = RawCapture {
            id: "cap-null-ctx".into(),
            original_content: "idea".into(),
            captured_at: "2026-09-11T15:01:00Z".into(),
            source_kind: None,
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        };
        db.insert_raw_capture(&capture).expect("insert");
        let loaded = db
            .get_raw_capture("cap-null-ctx")
            .expect("query")
            .expect("found");
        assert_eq!(loaded, capture);

        // No AI row required — query metadata count is zero.
        let count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM capture_ai_metadata WHERE capture_id = ?1",
                params!["cap-null-ctx"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn missing_id_returns_none() {
        let db = Database::open_in_memory().expect("open");
        let loaded = db.get_raw_capture("does-not-exist").expect("query");
        assert!(loaded.is_none());
    }

    #[test]
    fn original_content_preserved_exactly() {
        let db = Database::open_in_memory().expect("open");
        let content = "Unicode ✓  and\ttabs";
        let capture = RawCapture {
            id: "cap-exact".into(),
            original_content: content.into(),
            captured_at: "2026-09-11T15:02:00Z".into(),
            source_kind: None,
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        };
        db.insert_raw_capture(&capture).unwrap();
        let loaded = db.get_raw_capture("cap-exact").unwrap().unwrap();
        assert_eq!(loaded.original_content, content);
    }
}
