-- Migration identity: v004_classification
-- Lightweight AI processing status for classification (issue #16).

ALTER TABLE capture_ai_metadata ADD COLUMN processing_status TEXT;
-- Suggested values: classified | failed (NULL = not yet classified via #16)
