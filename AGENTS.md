# AGENTS.md

## Purpose

Local-first personal memory tool: one global shortcut captures useful on-screen/selection context; local GPU AI organizes, connects, and resurfaces it later. Principle: **capture first, organize later**. Not a generic notes app.

Product direction: `_docs/PLAN.md`.

## Stack direction (planned features)

Option A: Tauri 2 (Rust + web UI), SQLite + FTS5, local vectors (sqlite-vec or LanceDB), Ollama for local LLM/embeddings, MV3 browser extension. Scaffold exists; most features are not implemented. See `_docs/tasks.md` and `_docs/architecture.md`.

## Repository state

Tauri 2 + React + TypeScript scaffold at the repo root (`src/`, `src-tauri/`). Most product features are still unimplemented; see `_docs/architecture.md`.

## Commands

Documented in `README.md`. Use only these project commands:

```bash
npm install
npm run tauri dev
npm run tauri build
cargo test --manifest-path src-tauri/Cargo.toml
```

Do not invent additional npm/cargo scripts. There is no frontend unit-test command yet.

## Task management (do not replace)

- **Do not** create another task-management system or duplicate the backlog.
- `_docs/tasks.md` — backlog overview and issue index.
- **GitHub Issues** — source of truth for individual implementation tasks and acceptance criteria (Goal + Description on each issue).
- Before implementing: identify the relevant issue, read it fully, and treat Goal/Description as acceptance criteria.
- Work on **one task at a time**. Do not start unrelated work unless explicitly requested.

Workflow details: `_docs/process.md`.

## How coding agents should work

1. Confirm the target GitHub issue (from the user or `_docs/tasks.md`).
2. Read the issue Goal and Description before changing code.
3. Load only the `_docs/` files relevant to that task (see below).
4. Keep the diff scoped to that issue.
5. Test before calling the work done (see `_docs/testing-guidelines.md`).
6. Update docs when decisions change; commit completed work regularly.

## Core development rules

- Prefer local-first, privacy-preserving behavior; default AI/network calls stay on localhost.
- Preserve raw captures immutably; AI metadata must never overwrite originals.
- Minimize capture friction (one shortcut; no mandatory tagging/folder UI at capture time).
- Do not invent completed features, APIs, dependencies, or commands.
- Do not add dependencies unless required by the active issue.
- Keep `AGENTS.md` concise; put detail in `_docs/` and load those docs only when relevant.

## Architecture principles

- Desktop app owns global shortcut and local knowledge system; extension enriches browser capture.
- Flow: capture → raw storage → background AI pipeline → search / connections / resurfacing / assistant.
- Distinguish **planned** vs **implemented** architecture in `_docs/architecture.md`. Do not claim code exists that is not in the repo.

## Documents (`_docs/`)

| Doc | When to read |
|-----|----------------|
| `PLAN.md` | Product vision and requirements |
| `tasks.md` | Backlog overview / issue index |
| `process.md` | How to pick and finish a task |
| `architecture.md` | Planned vs current structure |
| `testing-guidelines.md` | How to verify work |
| `design-system.md` | UI principles (when touching UI) |

Issues: https://github.com/hvmdvvn/personal-memory-tool/issues
