# Personal Memory Tool

Local-first personal memory tool (Tauri 2 + React + TypeScript). Product direction lives in `_docs/PLAN.md`.

## Prerequisites

- [Node.js](https://nodejs.org/) (npm)
- [Rust](https://www.rust-lang.org/tools/install) (rustc + cargo)
- Tauri 2 system dependencies for your OS: https://v2.tauri.app/start/prerequisites/

## Setup

```bash
npm install
```

## Develop

```bash
npm run tauri dev
```

## Test

Rust unit tests (from repo root):

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Or from `src-tauri/`:

```bash
cargo test
```

There is no frontend unit-test script yet; do not invent one until a harness is added.

## Build

```bash
npm run tauri build
```

## Global shortcut

Default hotkey: **Ctrl+Shift+Space** (defined as `DEFAULT_GLOBAL_SHORTCUT` in `src-tauri/src/shortcut.rs`).

To change it: update that constant **and** the matching `Shortcut::new(...)` / `sc.matches(...)` wiring in `src-tauri/src/lib.rs` so they stay in sync. A config-file remap lands later (issue #28).

On press, the app logs `[shortcut] capture-shortcut-ack` and emits the Tauri event `capture-shortcut` with payload `capture-shortcut-ack`. No capture is saved yet (see issue #9).

### Manual test (Windows)

1. Run `npm run tauri dev` and wait until the window is up.
2. Focus another application (browser, Notepad, etc.).
3. Press **Ctrl+Shift+Space**.
4. Confirm the stub ack in the terminal (`[shortcut] capture-shortcut-ack`) and/or that the `capture-shortcut` event fires if you listen in the UI.
5. If nothing happens, another app may already own that hotkey—check the terminal for `failed to register`.

## Docs

See `_docs/` and `AGENTS.md` for architecture, process, and agent guidance.
