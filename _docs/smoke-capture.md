# Smoke path — capture → Inbox (Windows)

Session checklist for the primary happy path. Automated coverage lives in `cargo test --manifest-path src-tauri/Cargo.toml` (clipboard/orchestrate/list_recent tests). Real **global hotkey** registration cannot be fully automated in CI—mark those steps **manual**.

## Prerequisites

- Node.js + npm, Rust toolchain, Tauri Windows prerequisites
- From repo root: `npm install`
- Optional for AI paths: Ollama running locally (`ollama_health`)

## Automated / headless (CI-friendly)

These are covered by Rust unit tests (no UI, no real hotkey):

1. Open DB + migrate (`Database::open` / in-memory migrate tests)
2. Insert raw capture → `list_recent_captures` orders newest first
3. Clipboard capture path with mocked clipboard (`capture` / `orchestrate` tests)
4. Inbox data contract: `CaptureSummary` fields used by `src/App.tsx`

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Manual checklist (`tauri dev`)

1. `npm run tauri dev` — window opens; terminal shows shortcut registration or a conflict warning.
2. Confirm app data DB will be created under the Tauri app data dir as `memory.sqlite` (media under `media/`).
3. **Manual:** Copy text to the clipboard, focus another app, press **Ctrl+Shift+Space**.
4. Confirm log line `[shortcut] capture_now ok id=…` (or use invoke `capture_clipboard` / `capture_now` from the webview console if hotkey is blocked).
5. In the Inbox view, click **Refresh** (or wait for `capture-shortcut` event) and confirm the new snippet appears.
6. Optional: Search / Assistant / Today views still load without error.

## Manual-only notes

| Step | Why manual |
|------|------------|
| Global hotkey press | OS-level registration; conflicts with other apps |
| Real clipboard from another process | Test harness mocks clipboard |
| Screenshot of primary monitor | Needs interactive desktop session |

## Pass criteria

- App starts without panic
- At least one capture visible in Inbox after shortcut **or** command capture
- `cargo test --manifest-path src-tauri/Cargo.toml` is green
