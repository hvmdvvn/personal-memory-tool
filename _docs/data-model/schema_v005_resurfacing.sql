-- Migration identity: v005_resurfacing
-- Daily resurfacing picks keyed by UTC calendar day (issue #24).

CREATE TABLE IF NOT EXISTS today_resurfacing (
    day TEXT NOT NULL,
    -- UTC calendar day YYYY-MM-DD
    capture_id TEXT NOT NULL
        REFERENCES captures(id) ON DELETE CASCADE,
    rank INTEGER NOT NULL,
    picked_at TEXT NOT NULL,
    PRIMARY KEY (day, capture_id)
);

CREATE INDEX IF NOT EXISTS idx_today_resurfacing_day ON today_resurfacing(day);
