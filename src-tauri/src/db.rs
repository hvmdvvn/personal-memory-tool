//! Local SQLite persistence for raw captures.
//!
//! Migrations: `v001_captures`, `v002_captures_fts`, `v003_embeddings`, `v004_classification`,
//! `v005_resurfacing`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Migration identity recorded in `schema_migrations`.
pub const MIGRATION_V001_CAPTURES: &str = "v001_captures";
pub const MIGRATION_V002_CAPTURES_FTS: &str = "v002_captures_fts";
pub const MIGRATION_V003_EMBEDDINGS: &str = "v003_embeddings";
pub const MIGRATION_V004_CLASSIFICATION: &str = "v004_classification";
pub const MIGRATION_V005_RESURFACING: &str = "v005_resurfacing";

const V001_SQL: &str = include_str!("../../_docs/data-model/schema_v001.sql");
const V002_SQL: &str = include_str!("../../_docs/data-model/schema_v002_fts.sql");
const V003_SQL: &str = include_str!("../../_docs/data-model/schema_v003_embeddings.sql");
const V004_SQL: &str = include_str!("../../_docs/data-model/schema_v004_classification.sql");
const V005_SQL: &str = include_str!("../../_docs/data-model/schema_v005_resurfacing.sql");

const SNIPPET_MAX_CHARS: usize = 160;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSummary {
    pub id: String,
    pub snippet: String,
    pub captured_at: String,
    pub source_kind: Option<String>,
    pub source_app: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiClassification {
    pub capture_id: String,
    pub content_type: Option<String>,
    pub confidence: Option<f64>,
    pub processing_status: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiEnrichment {
    pub capture_id: String,
    pub topics_json: Option<String>,
    pub keywords_json: Option<String>,
    pub entities_json: Option<String>,
    pub short_description: Option<String>,
    pub processing_status: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResurfaceItem {
    pub id: String,
    pub snippet: String,
    pub captured_at: String,
    pub source_kind: Option<String>,
    pub source_app: Option<String>,
    pub content_type: Option<String>,
    pub short_description: Option<String>,
    pub rank: i64,
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

        self.apply_migration(MIGRATION_V001_CAPTURES, V001_SQL)?;
        self.apply_migration(MIGRATION_V002_CAPTURES_FTS, V002_SQL)?;
        self.apply_migration(MIGRATION_V003_EMBEDDINGS, V003_SQL)?;
        self.apply_migration(MIGRATION_V004_CLASSIFICATION, V004_SQL)?;
        self.apply_migration(MIGRATION_V005_RESURFACING, V005_SQL)?;

        // Ensure FKs remain on after any SQL that may have set them.
        self.configure()?;
        Ok(())
    }

    fn apply_migration(&self, id: &str, sql: &str) -> rusqlite::Result<()> {
        if !self.migration_applied(id)? {
            self.conn.execute_batch(sql)?;
            self.conn.execute(
                "INSERT INTO schema_migrations (id, applied_at) VALUES (?1, datetime('now'));",
                params![id],
            )?;
        }
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

    pub fn count_raw_captures(&self) -> rusqlite::Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM captures", [], |row| row.get(0))
    }

    pub fn count_ai_metadata_for(&self, capture_id: &str) -> rusqlite::Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM capture_ai_metadata WHERE capture_id = ?1",
            params![capture_id],
            |row| row.get(0),
        )
    }

    /// Upsert embedding vector blob for a capture; does not touch captures.original_content.
    pub fn upsert_embedding(
        &self,
        capture_id: &str,
        model: &str,
        dims: i64,
        vector_blob: &[u8],
        created_at: &str,
        embedding_ref: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO capture_embeddings (capture_id, model, dims, vector_blob, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(capture_id) DO UPDATE SET
               model = excluded.model,
               dims = excluded.dims,
               vector_blob = excluded.vector_blob,
               created_at = excluded.created_at",
            params![capture_id, model, dims, vector_blob, created_at],
        )?;

        self.conn.execute(
            "INSERT INTO capture_ai_metadata (
                capture_id, content_type, confidence, topics_json, keywords_json, entities_json,
                short_description, embedding_ref, updated_at
             ) VALUES (?1, NULL, NULL, NULL, NULL, NULL, NULL, ?2, ?3)
             ON CONFLICT(capture_id) DO UPDATE SET
               embedding_ref = excluded.embedding_ref,
               updated_at = excluded.updated_at",
            params![capture_id, embedding_ref, created_at],
        )?;
        Ok(())
    }

    pub fn get_embedding_blob(&self, capture_id: &str) -> rusqlite::Result<Option<Vec<u8>>> {
        self.conn
            .query_row(
                "SELECT vector_blob FROM capture_embeddings WHERE capture_id = ?1",
                params![capture_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn get_embedding_ref(&self, capture_id: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT embedding_ref FROM capture_ai_metadata WHERE capture_id = ?1",
                params![capture_id],
                |row| row.get(0),
            )
            .optional()
    }

    /// Upsert classification fields only; preserves embedding_ref and enrichment columns.
    pub fn upsert_classification(
        &self,
        capture_id: &str,
        content_type: Option<&str>,
        confidence: Option<f64>,
        processing_status: &str,
        updated_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO capture_ai_metadata (
                capture_id, content_type, confidence, topics_json, keywords_json, entities_json,
                short_description, embedding_ref, updated_at, processing_status
             ) VALUES (?1, ?2, ?3, NULL, NULL, NULL, NULL, NULL, ?4, ?5)
             ON CONFLICT(capture_id) DO UPDATE SET
               content_type = excluded.content_type,
               confidence = excluded.confidence,
               processing_status = excluded.processing_status,
               updated_at = excluded.updated_at",
            params![
                capture_id,
                content_type,
                confidence,
                updated_at,
                processing_status
            ],
        )?;
        Ok(())
    }

    pub fn get_ai_classification(
        &self,
        capture_id: &str,
    ) -> rusqlite::Result<Option<AiClassification>> {
        self.conn
            .query_row(
                "SELECT capture_id, content_type, confidence, processing_status, updated_at
                 FROM capture_ai_metadata WHERE capture_id = ?1",
                params![capture_id],
                |row| {
                    Ok(AiClassification {
                        capture_id: row.get(0)?,
                        content_type: row.get(1)?,
                        confidence: row.get(2)?,
                        processing_status: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()
    }

    /// Upsert enrichment fields only; preserves classification and embedding_ref.
    pub fn upsert_enrichment(
        &self,
        capture_id: &str,
        topics_json: &str,
        keywords_json: &str,
        entities_json: &str,
        short_description: &str,
        processing_status: &str,
        updated_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO capture_ai_metadata (
                capture_id, content_type, confidence, topics_json, keywords_json, entities_json,
                short_description, embedding_ref, updated_at, processing_status
             ) VALUES (?1, NULL, NULL, ?2, ?3, ?4, ?5, NULL, ?6, ?7)
             ON CONFLICT(capture_id) DO UPDATE SET
               topics_json = excluded.topics_json,
               keywords_json = excluded.keywords_json,
               entities_json = excluded.entities_json,
               short_description = excluded.short_description,
               processing_status = excluded.processing_status,
               updated_at = excluded.updated_at",
            params![
                capture_id,
                topics_json,
                keywords_json,
                entities_json,
                short_description,
                updated_at,
                processing_status
            ],
        )?;
        Ok(())
    }

    pub fn get_ai_enrichment(&self, capture_id: &str) -> rusqlite::Result<Option<AiEnrichment>> {
        self.conn
            .query_row(
                "SELECT capture_id, topics_json, keywords_json, entities_json,
                        short_description, processing_status, updated_at
                 FROM capture_ai_metadata WHERE capture_id = ?1",
                params![capture_id],
                |row| {
                    Ok(AiEnrichment {
                        capture_id: row.get(0)?,
                        topics_json: row.get(1)?,
                        keywords_json: row.get(2)?,
                        entities_json: row.get(3)?,
                        short_description: row.get(4)?,
                        processing_status: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
    }

    /// Soft-fail marker for AI pipeline errors (does not touch raw captures).
    pub fn mark_ai_processing_failed(
        &self,
        capture_id: &str,
        updated_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO capture_ai_metadata (
                capture_id, content_type, confidence, topics_json, keywords_json, entities_json,
                short_description, embedding_ref, updated_at, processing_status
             ) VALUES (?1, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?2, 'failed')
             ON CONFLICT(capture_id) DO UPDATE SET
               processing_status = 'failed',
               updated_at = excluded.updated_at",
            params![capture_id, updated_at],
        )?;
        Ok(())
    }

    /// Captures with `captured_at` strictly before `cutoff_iso` (lexicographic ISO compare).
    pub fn list_captures_older_than(
        &self,
        cutoff_iso: &str,
    ) -> rusqlite::Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, captured_at FROM captures
             WHERE captured_at < ?1
             ORDER BY captured_at ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![cutoff_iso], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Replace the stored set for a UTC day.
    pub fn replace_today_resurfacing(
        &self,
        day: &str,
        capture_ids: &[String],
        picked_at: &str,
    ) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM today_resurfacing WHERE day = ?1", params![day])?;
        for (rank, id) in capture_ids.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO today_resurfacing (day, capture_id, rank, picked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![day, id, rank as i64, picked_at],
            )?;
        }
        Ok(())
    }

    pub fn get_today_resurfacing(
        &self,
        day: &str,
    ) -> rusqlite::Result<Option<Vec<ResurfaceItem>>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.capture_id, c.original_content, c.captured_at, c.source_kind, c.source_app,
                    m.content_type, m.short_description, t.rank
             FROM today_resurfacing t
             JOIN captures c ON c.id = t.capture_id
             LEFT JOIN capture_ai_metadata m ON m.capture_id = t.capture_id
             WHERE t.day = ?1
             ORDER BY t.rank ASC",
        )?;
        let rows = stmt.query_map(params![day], |row| {
            let content: String = row.get(1)?;
            Ok(ResurfaceItem {
                id: row.get(0)?,
                snippet: truncate_snippet(&content, SNIPPET_MAX_CHARS),
                captured_at: row.get(2)?,
                source_kind: row.get(3)?,
                source_app: row.get(4)?,
                content_type: row.get(5)?,
                short_description: row.get(6)?,
                rank: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        if out.is_empty() {
            // Distinguish "no row for day" vs empty pick: check existence.
            let count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM today_resurfacing WHERE day = ?1",
                params![day],
                |r| r.get(0),
            )?;
            if count == 0 {
                return Ok(None);
            }
        }
        Ok(Some(out))
    }

    /// All stored embeddings as (capture_id, little-endian f32 blob).
    pub fn list_embedding_blobs(&self) -> rusqlite::Result<Vec<(String, Vec<u8>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT capture_id, vector_blob FROM capture_embeddings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn summary_for_capture(&self, id: &str) -> rusqlite::Result<Option<CaptureSummary>> {
        Ok(self.get_raw_capture(id)?.map(|c| CaptureSummary {
            id: c.id,
            snippet: truncate_snippet(&c.original_content, SNIPPET_MAX_CHARS),
            captured_at: c.captured_at,
            source_kind: c.source_kind,
            source_app: c.source_app,
        }))
    }

    /// Recent captures for Inbox, newest first.
    pub fn list_recent_captures(&self, limit: i64) -> rusqlite::Result<Vec<CaptureSummary>> {
        let limit = if limit <= 0 { 50 } else { limit };
        let mut stmt = self.conn.prepare(
            "SELECT id, original_content, captured_at, source_kind, source_app
             FROM captures
             ORDER BY captured_at DESC, id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let captured_at: String = row.get(2)?;
            let source_kind: Option<String> = row.get(3)?;
            let source_app: Option<String> = row.get(4)?;
            Ok(CaptureSummary {
                id,
                snippet: truncate_snippet(&content, SNIPPET_MAX_CHARS),
                captured_at,
                source_kind,
                source_app,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Exact/keyword search via FTS5. Independent of semantic search.
    pub fn search_exact(&self, query: &str) -> rusqlite::Result<Vec<CaptureSummary>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.original_content, c.captured_at, c.source_kind, c.source_app
             FROM captures_fts
             JOIN captures c ON c.rowid = captures_fts.rowid
             WHERE captures_fts MATCH ?1
             ORDER BY rank",
        )?;

        let rows = stmt.query_map(params![trimmed], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let captured_at: String = row.get(2)?;
            let source_kind: Option<String> = row.get(3)?;
            let source_app: Option<String> = row.get(4)?;
            Ok(CaptureSummary {
                id,
                snippet: truncate_snippet(&content, SNIPPET_MAX_CHARS),
                captured_at,
                source_kind,
                source_app,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Test helper: whether foreign_keys pragma is on.
    pub fn foreign_keys_enabled(&self) -> rusqlite::Result<bool> {
        let v: i64 = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        Ok(v == 1)
    }
}

fn truncate_snippet(content: &str, max_chars: usize) -> String {
    let mut chars = content.chars();
    let snippet: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{snippet}…")
    } else {
        snippet
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

    fn capture(id: &str, content: &str) -> RawCapture {
        RawCapture {
            id: id.into(),
            original_content: content.into(),
            captured_at: "2026-09-11T16:00:00Z".into(),
            source_kind: None,
            source_app: None,
            source_title: None,
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
        assert!(db.migration_applied(MIGRATION_V002_CAPTURES_FTS).unwrap());
        assert!(db.migration_applied(MIGRATION_V003_EMBEDDINGS).unwrap());
        assert!(db.migration_applied(MIGRATION_V004_CLASSIFICATION).unwrap());
        assert!(db.migration_applied(MIGRATION_V005_RESURFACING).unwrap());
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

    #[test]
    fn search_exact_hits_and_misses() {
        let db = Database::open_in_memory().expect("open");
        db.insert_raw_capture(&capture("a", "Discipline is doing hard things daily"))
            .unwrap();
        db.insert_raw_capture(&capture("b", "A recipe for chocolate cake"))
            .unwrap();

        let hits = db.search_exact("discipline").expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
        assert!(hits[0].snippet.to_lowercase().contains("discipline"));
        assert_eq!(hits[0].captured_at, "2026-09-11T16:00:00Z");

        let misses = db.search_exact("quantum").expect("search");
        assert!(misses.is_empty());
    }

    #[test]
    fn search_exact_empty_query_returns_empty() {
        let db = Database::open_in_memory().expect("open");
        db.insert_raw_capture(&capture("a", "something searchable"))
            .unwrap();
        assert!(db.search_exact("").unwrap().is_empty());
        assert!(db.search_exact("   ").unwrap().is_empty());
    }

    #[test]
    fn list_recent_captures_orders_and_limits() {
        let db = Database::open_in_memory().expect("open");
        db.insert_raw_capture(&RawCapture {
            id: "old".into(),
            original_content: "older item".into(),
            captured_at: "2026-01-01T00:00:00Z".into(),
            source_kind: Some("clipboard".into()),
            source_app: Some("A".into()),
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        })
        .unwrap();
        db.insert_raw_capture(&RawCapture {
            id: "new".into(),
            original_content: "newer item".into(),
            captured_at: "2026-06-01T00:00:00Z".into(),
            source_kind: Some("screenshot".into()),
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        })
        .unwrap();
        db.insert_raw_capture(&RawCapture {
            id: "mid".into(),
            original_content: "middle item".into(),
            captured_at: "2026-03-01T00:00:00Z".into(),
            source_kind: None,
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        })
        .unwrap();

        let page = db.list_recent_captures(2).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].id, "new");
        assert_eq!(page[1].id, "mid");
        assert_eq!(page[0].source_kind.as_deref(), Some("screenshot"));
    }
}
