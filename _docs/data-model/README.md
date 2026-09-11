# Data model — v001 captures

**Migration identity:** `v001_captures`  
**Schema file:** [`schema_v001.sql`](./schema_v001.sql)

This design artifact is from issue [#2](https://github.com/hvmdvvn/personal-memory-tool/issues/2). Issue [#3](https://github.com/hvmdvvn/personal-memory-tool/issues/3) applies it via `src-tauri/src/db.rs` (migration identity `v001_captures`).

## Invariant

**AI processing must never overwrite or replace original capture content.**

- `captures` holds the immutable raw memory.
- `capture_ai_metadata` holds derived fields only.
- Enrichment, classification, and embeddings update (or insert) metadata rows; they do not rewrite `captures.original_content` or other raw provenance columns.

## Table: `captures` (raw)

| Column | Meaning |
|--------|---------|
| `id` | Stable unique id for the capture (TEXT primary key; format chosen at implementation, e.g. UUID). |
| `original_content` | Exact text as captured. Required. Not an AI summary. |
| `captured_at` | When the item was saved (ISO-8601 UTC text). |
| `source_kind` | Optional coarse origin (`clipboard`, `browser`, `screenshot`, `unknown`, …). |
| `source_app` | Optional foreground application name. |
| `source_title` | Optional window or page title. |
| `source_url` | Optional URL (browser). |
| `source_extra` | Optional JSON for extra context without schema churn. |
| `media_path` | Optional filesystem path to related media (e.g. screenshot file). |

All source/media columns are **nullable** so a capture can be stored when context is unavailable.

A row in `captures` does **not** require a matching `capture_ai_metadata` row.

## Table: `capture_ai_metadata` (AI-only)

| Column | Meaning |
|--------|---------|
| `capture_id` | FK to `captures.id`. Primary key → at most one metadata row per capture. |
| `content_type` | AI classification (see below). Nullable until classified. |
| `confidence` | Optional model confidence for the classification. |
| `topics_json` | JSON array of topic strings. |
| `keywords_json` | JSON array of keyword strings. |
| `entities_json` | JSON array of entity strings (or simple objects serialized as JSON). |
| `short_description` | Short AI-written description; never replaces `original_content`. |
| `embedding_ref` | Opaque reference/slot for a vector produced later (#14). Not the vector bytes. |
| `updated_at` | Last time this metadata row was written. |

Absence of a row means “not yet processed” (or processing never started).

## Content types (`content_type`)

Aligned with `_docs/PLAN.md`. Stored as TEXT so new values can be added without a migration. Suggested values:

| Value | Intent |
|-------|--------|
| `book_quote` | Quote from a book |
| `idea` | Idea / thought |
| `note` | General note |
| `article` | Article |
| `web_link` | Web link |
| `video_post` | Video or social post |
| `conversation` | Conversation / message |
| `task` | Task |
| `general` | General information |

The user is **not** required to set type at capture time; AI may fill `content_type` later.

## Intended migration application (#3)

1. Record applied migrations (mechanism chosen in #3).
2. Apply `v001_captures` exactly once per database (contents of `schema_v001.sql`).
3. Enable foreign keys on connections.
4. Only then implement insert raw capture / read by id (still without AI overwrite of raw columns).

## Explicitly not in v001

- FTS5 virtual tables (#4)
- Concrete vector storage (#14)
- Capture/UI commands (#6–#10)
- Prompt-version or full AI job status machines (#16 / #17)
