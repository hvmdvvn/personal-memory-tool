# Backlog — Personal Memory Tool

Stack decision: **Option A** — Tauri 2 (Rust + web UI), SQLite + FTS5, sqlite-vec or LanceDB, Ollama for local GPU LLM/embeddings, Chrome/Firefox MV3 browser extension.

Each task is sized for one working session and written so it can be handed off without reading the rest of this file.

---

## 1. Scaffold empty Tauri project with a passing test
Goal: Create the Option A project skeleton and prove CI-local confidence with one green test.
Description: Initialize a Tauri 2 app (Rust backend + chosen web UI, React or Svelte) under the repo root or an agreed app directory. Add the minimal unit-test setup for the Rust side (and optionally the frontend) and commit a trivial passing test such as `assert_eq!(2 + 2, 4)` or a health-check helper. Document how to run `cargo test` (and any frontend test command) in a short README section so the next person can verify green without guessing.

## 2. Define immutable capture and AI-metadata data model
Goal: Specify how raw captures and AI-generated fields are stored without conflating them.
Description: Design the SQLite schema (or equivalent typed structs + migration plan) for a personal memory tool where each item keeps original content, timestamp, and source/context exactly as captured, with AI classification, topics, keywords, embeddings refs, and descriptions in separate tables/columns. Document field meanings and invariants (AI never overwrites original). Deliver schema SQL or Rust types plus a short note; no UI or capture logic required.

## 3. Implement SQLite persistence with migrations
Goal: Stand up local SQLite storage that creates and migrates the capture schema.
Description: Using the Option A stack, wire SQLite into the Tauri/Rust backend with a migration runner that applies the capture + AI-metadata schema on first launch. Expose a minimal Rust API to insert a raw capture and read it back by id, preserving original fields unchanged. Include tests that open a temp DB, migrate, insert, and round-trip a capture.

## 4. Add full-text search over stored captures
Goal: Enable exact/keyword search via SQLite FTS5 (or equivalent) on capture content.
Description: Extend the local DB so users can search by matching words or phrases in original capture text and useful text fields. Keep the feature independent of semantic/vector search. Provide a Rust (or Tauri command) API like `search_exact(query) -> Vec<CaptureSummary>` and tests with seeded rows that assert expected hits and misses.

## 5. Global shortcut registration in the desktop app
Goal: Register one OS-level keyboard shortcut that the Tauri app receives while running.
Description: In the Tauri 2 desktop app, register a configurable global hotkey (sensible default documented) that fires even when the app is not focused. On trigger, emit an internal event or invoke a stub handler that logs or returns a fixed ack—no real capture yet. Document how to change the shortcut and test manually on Windows; add an automated test only where the stack allows mocking the hotkey layer.

## 6. Capture clipboard text as a raw memory item
Goal: On demand, read the clipboard and store it as an immutable raw capture.
Description: Implement a Rust/Tauri command that reads current clipboard text, builds a raw capture (content, timestamp, source hint e.g. `clipboard`), and persists it via the existing SQLite layer without AI enrichment. Return the new capture id to the caller. Tests may mock clipboard access; include at least one test for empty clipboard behavior.

## 7. Capture active-window context metadata
Goal: Record which application (and title when available) was focused at capture time.
Description: Using Windows-friendly APIs from the Rust backend, resolve the foreground process name and window title at capture time and attach them as source/context on the raw capture—not as AI guesses. Integrate with the clipboard (or stub) capture path so context is stored alongside content. Handle permission/API failure by storing null/empty context without failing the whole capture; cover the failure path in tests or a documented manual check.

## 8. Screenshot capture path for on-screen context
Goal: Save a screenshot of the active monitor or window as part of a raw capture when requested.
Description: Add a capture mode that grabs image bytes (PNG), stores them on disk under a local data directory, and references the path from the raw capture record while keeping any text fields separate. Do not run OCR or AI in this task. Document storage location and cleanup expectations; add a test that writes a tiny fake image fixture through the same storage helper.

## 9. Stub capture orchestration command
Goal: Provide one Tauri command that represents “global shortcut capture” and chooses a simple capture strategy.
Description: Implement `capture_now` (name flexible) that, for now, prefers non-empty clipboard text, else records window context with an optional screenshot flag, and always writes one raw capture row. Wire it so the global shortcut can call this command. No classification or embeddings. Tests should cover “clipboard wins” vs “fallback” with mocks.

## 10. Minimal inbox UI for recent captures
Goal: Show recently saved items in the Tauri web UI so capture results are visible.
Description: Build a simple Inbox view listing recent captures (timestamp, truncated content, source/app). Load data via Tauri commands from SQLite. No editing, tagging, or AI panels required—just read-only list + refresh after capture if easy. Keep styling minimal; this is a functional shell for the home dashboard’s Inbox area.

## 11. Browser extension skeleton with one message to the desktop app
Goal: Create an MV3 extension that can send a ping (or sample payload) to the running desktop app.
Description: Scaffold a Chrome-compatible Manifest V3 extension (Firefox notes optional) with a background/service worker capable of talking to the Tauri app via native messaging or authenticated localhost IPC—pick one approach and document setup steps. Implement only a “ping” or “save sample text” path that the desktop acknowledges. No full page scraping yet; include extension README for loading unpacked and pairing with the app.

## 12. Browser capture: URL, title, and selection
Goal: Let the extension send page URL, title, and selected text into the desktop raw-capture store.
Description: On an extension action or message, collect `url`, `title`, and `window.getSelection()` text from the active tab and POST/send them to the desktop capture API as a raw capture with browser source context. Preserve originals exactly; do not classify. Document required host permissions. Provide a short manual test checklist (select text on a page → appear in Inbox).

## 13. Ollama connectivity health check
Goal: Verify the app can reach a local Ollama instance and report model availability.
Description: Add a Rust/Tauri command that checks Ollama’s local HTTP API (default `localhost:11434`), lists available models or probes a configured model name, and returns a clear ok/error status for the UI or logs. Do not run classification yet. Document required Ollama install and env/config for base URL; test against a mock HTTP server so CI works without a GPU.

## 14. Local embedding generation for a single capture
Goal: Produce and store an embedding vector for one capture’s text using a local model.
Description: Given a capture id, read its original text, call Ollama (or configured embed model) to get a vector, and store it in the chosen vector layer (sqlite-vec or LanceDB) keyed to that capture—without modifying original content. Document the default embed model name. Tests should mock the embed HTTP response and assert the vector row is linked to the capture id.

## 15. Semantic similarity search API
Goal: Return captures similar to a natural-language query using stored embeddings.
Description: Implement `search_semantic(query, limit)` that embeds the query with the same local embed model, runs nearest-neighbor search over stored vectors, and returns capture summaries with scores. Keep this separate from FTS. Seed or mock vectors in tests to assert ranking order for a tiny fixture set.

## 16. Background classification of a raw capture
Goal: Classify a capture’s type (quote, idea, article, task, etc.) via local LLM without altering originals.
Description: Add an async/background job or command that sends original text (and light context) to Ollama with a constrained prompt/schema, writes type + confidence into AI-metadata tables only, and marks processing status. Idempotent re-run should update AI fields, not raw content. Mock LLM responses in unit tests; document the allowed type enum.

## 17. AI metadata enrichment (topics, keywords, short description)
Goal: Generate secondary metadata for a capture using the local LLM.
Description: Extend the AI pipeline step to produce topics, keywords, entities (simple list), and a short description into AI-metadata storage. Fail soft on model errors (status = failed, raw intact). Include prompt versioning or a constant prompt id so later changes are traceable. Tests mock the model and assert metadata rows appear for a given capture id.

## 18. Unified search command (exact + semantic)
Goal: Offer one search entry point that combines keyword and semantic results.
Description: Implement a Tauri command that runs FTS and semantic search, merges/dedupes by capture id, and returns a ranked list with match reasons (`exact`, `semantic`, or both). No UI polish required beyond what the command returns; a thin UI search box is optional. Tests cover merge behavior with overlapping fixture hits.

## 19. Search UI with basic filters
Goal: Let the user search memories and filter by type and date range.
Description: Add a Search view in the Tauri web UI that calls the unified search API and supports simple filters (AI type when present, from/to date). Display results with timestamp, snippet, and type. Do not build the full dashboard—only Search. Include empty and error states for Ollama-down semantic degradation if feasible (exact-only fallback).

## 20. Related-items API for a single capture
Goal: For one capture, return other captures that appear related via embeddings (and optional shared metadata).
Description: Implement `related_to(capture_id, limit)` using vector similarity and optionally shared topics/keywords from AI metadata. Return summaries suitable for a “Related” panel. No graph visualization yet. Tests use a small embedded fixture set to ensure the source item is excluded and neighbors are ordered by score.

## 21. Personal Q&A command over stored memories
Goal: Answer a natural-language question using retrieved captures as the only knowledge source.
Description: Implement an assistant command: embed/search for relevant captures, stuff top-k originals into a local LLM prompt, and return an answer plus citation capture ids. Instruct the model to prefer stored knowledge and admit gaps. Mock Ollama in tests; document token/context limits and default chat model. Do not stream in this task unless already trivial.

## 22. AI Assistant chat panel in the UI
Goal: Provide a simple chat UI that calls the personal Q&A command and shows citations.
Description: Add an Assistant view with message list, input box, and display of cited capture ids (links to open/detail optional). Talk only to the local backend. Keep UI minimal and readable; no multi-session history DB required beyond what is needed for the current conversation in memory. Manual test: ask for a seeded quote and confirm a citation appears.

## 23. Connections view listing related memories
Goal: Surface automatic connections by browsing an item and seeing related captures.
Description: Build a Connections UI: select or open a capture, call `related_to`, and list related items with short explanations (similarity score and/or shared topics). Reuse Inbox/Search data loading patterns. No force-directed graph required in this task—list/detail is enough to match the plan’s Connections area.

## 24. Daily resurfacing picker job
Goal: Select a small set of older captures worth revisiting today.
Description: Implement a deterministic or lightly randomized daily job that picks N captures older than a threshold (e.g. 7+ days), preferring diversity by topic/type when AI metadata exists, and stores a “Today” set dated for the current day. Expose `get_today_resurfacing()` for the UI. Unit-test selection rules with a fixed clock and fixture DB.

## 25. Today view in the UI
Goal: Show the daily resurfaced memories on a simple Today screen.
Description: Add a Today view that loads `get_today_resurfacing()` and displays the chosen captures with age and short description/snippet. Include an empty state when the job has not run. Trigger or document how the daily job runs (on app start is fine). No contextual/active-window resurfacing in this task.

## 26. Contextual resurfacing stub based on active window
Goal: Prototype resurfacing suggestions from current app/window title text.
Description: Periodically or on demand, read active window title/process, run semantic (or keyword) search against memories, and return top suggestions as “contextual resurfacing” candidates. Store nothing destructive; this is a read-only suggestion API. Keep thresholds configurable. Tests mock window title and search backend.

## 27. End-to-end capture smoke path (shortcut → Inbox)
Goal: Document and automate as much as possible of the primary happy path on Windows.
Description: Write a session-sized integration checklist (and any harness tests that can run headlessly) covering: app starts, DB migrates, shortcut or command captures clipboard text, item appears via list API/Inbox. Note manual steps that cannot be automated (real global hotkey). This task delivers a smoke doc + any new glue tests, not new product features.

## 28. Config file for models, hotkey, and data directory
Goal: Centralize user-configurable paths and model names without hardcoding.
Description: Introduce a local config (file or Tauri store) for Ollama base URL, chat model, embed model, global hotkey, and data/DB directory. Load at startup with safe defaults; expose get/set commands. Do not build a full settings UI—CLI or a minimal settings form is enough. Tests verify defaults and override loading from a temp config file.

## 29. Packaging notes for Windows dev install
Goal: Make it clear how a developer runs the desktop app, extension, and Ollama together.
Description: Write developer setup documentation covering Rust/Node toolchain, `tauri dev`, loading the unpacked extension, installing/running Ollama with required models, and where SQLite/files live. Include troubleshooting for common Windows permission/hotkey issues. No code changes required unless a missing script blocks the documented flow—prefer docs-only.

## 30. Privacy and local-only networking guardrails
Goal: Ensure capture and AI paths do not call cloud endpoints by default.
Description: Audit commands and clients so LLM/embed HTTP goes only to the configured local Ollama base URL; document that cloud providers are out of scope for v1. Add a startup warning if config points at a non-loopback host. Tests assert the default config is localhost and that the HTTP client is constructed from config. No telemetry features in this task.
