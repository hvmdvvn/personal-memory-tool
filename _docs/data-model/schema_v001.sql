-- Migration identity: v001_captures
-- Apply later via the migration runner in issue #3. Do not run from app code in #2.

PRAGMA foreign_keys = ON;

-- Immutable raw captures. Application code must not UPDATE original_content
-- (or other raw columns) as part of AI processing. AI output belongs in
-- capture_ai_metadata only.

CREATE TABLE IF NOT EXISTS captures (
    id TEXT PRIMARY KEY NOT NULL,
    -- Exact payload as captured (text). Never replaced by AI rewrites.
    original_content TEXT NOT NULL,
    -- UTC instant when the capture was saved, ISO-8601 text (e.g. 2026-09-11T14:00:00Z).
    captured_at TEXT NOT NULL,
    -- Optional provenance. All nullable: capture may lack context.
    source_kind TEXT,
    -- e.g. clipboard | browser | screenshot | unknown
    source_app TEXT,
    -- Foreground application name when known
    source_title TEXT,
    -- Window or page title when known
    source_url TEXT,
    -- Page URL when known (browser captures)
    source_extra TEXT,
    -- Optional JSON object for additional context without widening the table yet
    media_path TEXT
    -- Optional local path to an associated file (e.g. screenshot); bytes not stored in DB
);

-- AI-generated metadata. One optional row per capture (absent until processed).
-- Re-running AI may UPDATE this row; it must never mutate captures.original_content.

CREATE TABLE IF NOT EXISTS capture_ai_metadata (
    capture_id TEXT PRIMARY KEY NOT NULL
        REFERENCES captures(id) ON DELETE CASCADE,
    -- Extensible classification; suggested values listed in the design note.
    content_type TEXT,
    confidence REAL,
    -- JSON arrays of strings, e.g. ["leadership","habits"]
    topics_json TEXT,
    keywords_json TEXT,
    entities_json TEXT,
    short_description TEXT,
    -- Abstract embedding linkage for #14 (vector store undecided).
    -- Examples later: vector row id, file key, or external handle — opaque string.
    embedding_ref TEXT,
    -- When this metadata row was last written by AI (ISO-8601 UTC).
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_captures_captured_at ON captures(captured_at);
CREATE INDEX IF NOT EXISTS idx_ai_content_type ON capture_ai_metadata(content_type);
