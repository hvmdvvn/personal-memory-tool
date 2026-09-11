# Windows developer setup

How to run the desktop app, browser extension, and local Ollama together on Windows.

## Toolchain

1. Install [Node.js](https://nodejs.org/) (npm included).
2. Install [Rust](https://www.rust-lang.org/tools/install) (`rustc`, `cargo`).
3. Install [Tauri 2 Windows prerequisites](https://v2.tauri.app/start/prerequisites/) (WebView2, VS Build Tools / MSVC as documented upstream).

From the repo root:

```bash
npm install
npm run tauri dev
```

Production-ish build:

```bash
npm run tauri build
```

Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Data locations

Default (Tauri app data directory on Windows), typically under:

`%APPDATA%\<identifier>\`

- `memory.sqlite` — captures + AI metadata + FTS + embeddings + today resurfacing
- `media/` — screenshot PNGs
- `config.json` — optional overrides (models, Ollama URL, hotkey string, data_dir)

If `config.json` sets `data_dir`, that path is used instead for DB/media/config (see `config` module).

## Ollama (local AI)

1. Install [Ollama](https://ollama.com/) and ensure it listens on `http://127.0.0.1:11434`.
2. Pull models used by defaults:

```bash
ollama pull nomic-embed-text
ollama pull llama3.2
```

3. Optional: `OLLAMA_HOST=127.0.0.1:11434` or set `ollama_base_url` in `config.json`.
4. In-app: Tauri command `ollama_health`.

**Privacy:** v1 expects loopback-only Ollama. Cloud LLM providers are out of scope. Non-loopback URLs log a startup warning.

## Browser extension

See [`extension/README.md`](../extension/README.md).

1. Load the unpacked `extension/` folder in Chrome/Edge (`chrome://extensions` → Developer mode).
2. Desktop app must be running so loopback IPC on `127.0.0.1:17832` accepts authenticated requests.
3. Token is the shared local-dev token documented in `src-tauri/src/ipc.rs`.

## Global shortcut

Default: **Ctrl+Shift+Space** (`DEFAULT_GLOBAL_SHORTCUT` in `shortcut.rs`, mirrored in `lib.rs` registration).

### Troubleshooting

| Symptom | What to try |
|---------|-------------|
| Hotkey does nothing | Check terminal for `failed to register` — another app owns the chord |
| App won’t start / WebView errors | Reinstall WebView2 runtime; confirm MSVC build tools |
| Capture empty | Ensure clipboard has text, or allow screenshot fallback |
| Ollama commands fail | `ollama serve` / confirm port 11434; run `ollama_health` |
| Extension cannot ping | App running? Firewall blocking loopback? Token header correct? |
| DB missing migrations | Delete `memory.sqlite` only if you accept data loss; reopen app to remigrate |

## Smoke path

See [`smoke-capture.md`](./smoke-capture.md) for the capture → Inbox checklist.
