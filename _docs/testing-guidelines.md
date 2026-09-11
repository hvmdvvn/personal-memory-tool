# Testing guidelines

## Current state

Application scaffold exists. The **Rust** test harness is Cargo’s built-in `#[test]` support in `src-tauri`.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

No frontend unit-test framework or CI pipeline has been selected yet. Do **not** invent npm test scripts or CI until an issue explicitly adds them.

## Principles

- Every implementation issue should leave behind a way to verify its Goal—prefer automated tests when a harness exists; otherwise document a short manual checklist in the PR/commit notes for that issue only.
- Prefer tests that protect invariants from `PLAN.md`, especially:
  - Raw capture content is stored and round-tripped unchanged.
  - AI-generated fields live separately and never overwrite originals.
  - Search and retrieval return expected fixtures for exact matches (and later semantic cases).
  - Local-only defaults (no accidental cloud endpoints) once networking/AI clients exist.
- Prefer fast, deterministic unit/integration tests with mocks for OS APIs (clipboard, window title, screenshots) and for Ollama HTTP once those layers exist.
- Do not require a GPU or a live Ollama instance for default automated tests; mock external services.
- Manual checks are acceptable for OS-level behavior that cannot be automated yet (e.g. real global hotkeys on Windows); call them out explicitly rather than claiming automated coverage.

## When a harness exists

- Document the real test command(s) in the README and in `AGENTS.md`.
- Run the relevant automated tests before considering an issue complete.
- Add or update tests in the same change set as the behavior they cover, when practical.
