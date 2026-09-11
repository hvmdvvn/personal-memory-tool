-- Migration identity: v002_captures_fts
-- Full-text index over raw capture content (issue #4). Independent of semantic search.

CREATE VIRTUAL TABLE IF NOT EXISTS captures_fts USING fts5(
    original_content,
    content = 'captures',
    content_rowid = 'rowid'
);

CREATE TRIGGER IF NOT EXISTS captures_fts_ai AFTER INSERT ON captures BEGIN
    INSERT INTO captures_fts(rowid, original_content)
    VALUES (new.rowid, new.original_content);
END;

CREATE TRIGGER IF NOT EXISTS captures_fts_ad AFTER DELETE ON captures BEGIN
    INSERT INTO captures_fts(captures_fts, rowid, original_content)
    VALUES ('delete', old.rowid, old.original_content);
END;

CREATE TRIGGER IF NOT EXISTS captures_fts_au AFTER UPDATE OF original_content ON captures BEGIN
    INSERT INTO captures_fts(captures_fts, rowid, original_content)
    VALUES ('delete', old.rowid, old.original_content);
    INSERT INTO captures_fts(rowid, original_content)
    VALUES (new.rowid, new.original_content);
END;

-- Backfill FTS for any rows already present when this migration runs.
INSERT INTO captures_fts(rowid, original_content)
SELECT rowid, original_content FROM captures;
