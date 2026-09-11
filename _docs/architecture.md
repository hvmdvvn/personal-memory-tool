# Architecture

## Status legend

- **Planned** — Specified in `_docs/PLAN.md` and the Option A stack decision; not present as product code yet.
- **Implemented** — Actually exists in the repository today.

---

## Currently implemented architecture

What exists in the repository today:

- **Tauri 2 desktop shell** — `src-tauri/` (Rust), default template `greet` command
- **React + TypeScript + Vite UI** — `src/`
- **Scaffold-only UI** — template app screen; not the product Inbox/Search/Assistant surfaces
- **Rust unit tests** — trivial health/smoke tests in `src-tauri` (`cargo test`)
- **Data model design (v001)** — `_docs/data-model/` (`schema_v001.sql` + design note)
- **SQLite persistence** — `src-tauri/src/db.rs`: open/migrate `v001_captures` + `v002_captures_fts`, insert/get raw captures, `search_exact` (FTS5)
- **Global shortcut stub** — `Ctrl+Shift+Space` via `tauri-plugin-global-shortcut`; emits `capture-shortcut` ack (no capture yet)
- **Clipboard capture** — `capture_clipboard` Tauri command + `capture` module; stores raw row with `source_kind=clipboard`
- **Active-window context** — soft-fail `source_app` / `source_title` via Windows APIs (`window_context`)
- **Screenshot media** — PNG under `{app_data}/media/`, raw capture `media_path` + `capture_screenshot` command
- **Capture orchestration** — `capture_now` (clipboard preferred, screenshot/context fallback); wired to global shortcut
- **Inbox UI** — React Inbox lists recent captures via `list_recent_captures`; refreshes on shortcut event
- **Browser extension IPC** — MV3 `extension/` pings desktop on `127.0.0.1:17832` with shared token; `POST /capture` stores URL/title/selection
- **Ollama health** — `ollama_health` probes local `/api/tags` (mock-tested; default `127.0.0.1:11434`)
- **Embeddings** — `embed_capture` stores vectors in `capture_embeddings` (v003); default model `nomic-embed-text`
- **Semantic search** — `search_semantic` cosine k-NN over stored embeddings
- **Classification** — `classify_capture` via Ollama generate; writes `content_type`/`confidence`/`processing_status` (v004)
- **Enrichment** — `enrich_capture` topics/keywords/entities/short_description (`enrich_v1` prompt id)
- **Unified search** — `search_unified` merges FTS + semantic with `exact`/`semantic`/`both` reasons
- **Search UI** — Inbox/Search nav; query + type/date filters calling `search_unified`
- **Related items** — `related_to` cosine neighbors (optional topics/keywords boost)
- **Personal Q&A** — `ask_memories` retrieves top-k, prompts Ollama, returns answer + citations
- **Assistant / Connections UI** — chat panel + related-items browser in the web UI
- **Daily resurfacing** — `ensure_today_resurfacing` / `get_today_resurfacing` (v005); Today view
- **Contextual resurfacing** — `contextual_suggestions` from active window title/app (read-only)
- **Config + privacy** — `config.json` (get/set); loopback warning for non-local Ollama
- Planning and context docs under `_docs/`, plus agent entrypoints (`AGENTS.md`, `CLAUDE.md`)

Not implemented yet: capture engine, AI pipeline, browser extension, search, connections, resurfacing, or personal assistant.

---

## Planned architecture

### Product shape

- **Desktop application** — Owns the global shortcut and local knowledge system.
- **Browser extension** — Improves browser capture (URL, title, selection, page metadata) and talks to the desktop app.

### Planned stack (Option A)

Documented in `_docs/tasks.md`:

- Tauri 2 (Rust backend + web UI) — **scaffold present**
- SQLite + FTS5 for durable storage and exact search — **not implemented**
- sqlite-vec or LanceDB for embeddings / similarity — **not implemented**
- Ollama for local GPU LLM and embedding models — **not implemented**
- Chrome/Firefox Manifest V3 extension — **not implemented**

### Logical flow (from `PLAN.md`)

```text
                 USER
                   |
                   v
          Global Keyboard Shortcut
                   |
                   v
             Capture Engine
                   |
          +--------+--------+
          |                 |
          v                 v
     Screen/Selection    Context
       Extraction       Detection
          |                 |
          +--------+--------+
                   |
                   v
             Raw Capture
                   |
                   v
             Local Storage
                   |
                   v
          Background AI Pipeline
                   |
       +-----------+-----------+
       |           |           |
       v           v           v
   Classifier   Embeddings   Metadata
       |           |           |
       +-----------+-----------+
                   |
                   v
          Personal Knowledge DB
                   |
       +-----------+-----------+
       |           |           |
       v           v           v
    Search     Connections   Resurfacing
       |           |           |
       +-----------+-----------+
                   |
                   v
          Personal AI Assistant
```

### Planned responsibilities

| Area | Intent |
|------|--------|
| Capture engine | Global shortcut; selection/screen/clipboard/browser context; choose useful representation without user classification |
| Raw capture store | Immutable originals + timestamp + source/context |
| Background AI | Classification, metadata, embeddings—written beside originals, never replacing them |
| Knowledge DB | Queryable personal memory (exact + semantic) |
| Search / connections / resurfacing | Retrieval, related items, daily and contextual resurfacing |
| Personal AI assistant | Answers grounded in stored captures with citations |

### Data integrity principle

Original captured content remains exactly as captured. AI-generated metadata is separate. This must remain true as persistence and pipelines are implemented.

---

## Updating this document

When application features land, move pieces from **Planned** to **Implemented** and describe real modules, crates, and IPC boundaries. Do not list planned components as implemented.
