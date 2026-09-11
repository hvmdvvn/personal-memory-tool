# Backlog — Personal Memory Tool

Stack decision: **Option A** — Tauri 2 (Rust + web UI), SQLite + FTS5, sqlite-vec or LanceDB, Ollama for local GPU LLM/embeddings, Chrome/Firefox MV3 browser extension.

The session-sized backlog now lives as GitHub issues:

https://github.com/hvmdvvn/personal-memory-tool/issues

- [#1](https://github.com/hvmdvvn/personal-memory-tool/issues/1) Scaffold empty Tauri project with a passing test
- [#2](https://github.com/hvmdvvn/personal-memory-tool/issues/2) Define immutable capture and AI-metadata data model
- [#3](https://github.com/hvmdvvn/personal-memory-tool/issues/3) Implement SQLite persistence with migrations
- [#4](https://github.com/hvmdvvn/personal-memory-tool/issues/4) Add full-text search over stored captures
- [#5](https://github.com/hvmdvvn/personal-memory-tool/issues/5) Global shortcut registration in the desktop app
- [#6](https://github.com/hvmdvvn/personal-memory-tool/issues/6) Capture clipboard text as a raw memory item
- [#7](https://github.com/hvmdvvn/personal-memory-tool/issues/7) Capture active-window context metadata
- [#8](https://github.com/hvmdvvn/personal-memory-tool/issues/8) Screenshot capture path for on-screen context
- [#9](https://github.com/hvmdvvn/personal-memory-tool/issues/9) Stub capture orchestration command
- [#10](https://github.com/hvmdvvn/personal-memory-tool/issues/10) Minimal inbox UI for recent captures
- [#11](https://github.com/hvmdvvn/personal-memory-tool/issues/11) Browser extension skeleton with one message to the desktop app
- [#12](https://github.com/hvmdvvn/personal-memory-tool/issues/12) Browser capture: URL, title, and selection
- [#13](https://github.com/hvmdvvn/personal-memory-tool/issues/13) Ollama connectivity health check
- [#14](https://github.com/hvmdvvn/personal-memory-tool/issues/14) Local embedding generation for a single capture
- [#15](https://github.com/hvmdvvn/personal-memory-tool/issues/15) Semantic similarity search API
- [#16](https://github.com/hvmdvvn/personal-memory-tool/issues/16) Background classification of a raw capture
- [#17](https://github.com/hvmdvvn/personal-memory-tool/issues/17) AI metadata enrichment (topics, keywords, short description)
- [#18](https://github.com/hvmdvvn/personal-memory-tool/issues/18) Unified search command (exact + semantic)
- [#19](https://github.com/hvmdvvn/personal-memory-tool/issues/19) Search UI with basic filters
- [#20](https://github.com/hvmdvvn/personal-memory-tool/issues/20) Related-items API for a single capture
- [#21](https://github.com/hvmdvvn/personal-memory-tool/issues/21) Personal Q&A command over stored memories
- [#22](https://github.com/hvmdvvn/personal-memory-tool/issues/22) AI Assistant chat panel in the UI
- [#23](https://github.com/hvmdvvn/personal-memory-tool/issues/23) Connections view listing related memories
- [#24](https://github.com/hvmdvvn/personal-memory-tool/issues/24) Daily resurfacing picker job
- [#25](https://github.com/hvmdvvn/personal-memory-tool/issues/25) Today view in the UI
- [#26](https://github.com/hvmdvvn/personal-memory-tool/issues/26) Contextual resurfacing stub based on active window
- [#27](https://github.com/hvmdvvn/personal-memory-tool/issues/27) End-to-end capture smoke path (shortcut → Inbox)
- [#28](https://github.com/hvmdvvn/personal-memory-tool/issues/28) Config file for models, hotkey, and data directory
- [#29](https://github.com/hvmdvvn/personal-memory-tool/issues/29) Packaging notes for Windows dev install
- [#30](https://github.com/hvmdvvn/personal-memory-tool/issues/30) Privacy and local-only networking guardrails
