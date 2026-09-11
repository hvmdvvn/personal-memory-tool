-- Migration identity: v003_embeddings
-- Local float-vector storage keyed by capture (issue #14). Not sqlite-vec; app-side cosine later (#15).

CREATE TABLE IF NOT EXISTS capture_embeddings (
    capture_id TEXT PRIMARY KEY NOT NULL
        REFERENCES captures(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    dims INTEGER NOT NULL,
    -- Little-endian f32 components
    vector_blob BLOB NOT NULL,
    created_at TEXT NOT NULL
);
