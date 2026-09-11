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
- **Data model design (v001)** — `_docs/data-model/` (`schema_v001.sql` + design note); not applied by the app yet
- Planning and context docs under `_docs/`, plus agent entrypoints (`AGENTS.md`, `CLAUDE.md`)

Not implemented yet: capture engine, live SQLite migrations/CRUD, AI pipeline, browser extension, search, connections, resurfacing, or personal assistant.

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
