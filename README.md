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

## Docs

See `_docs/` and `AGENTS.md` for architecture, process, and agent guidance.
