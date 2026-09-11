# Browser extension (MV3)

Chrome-compatible Manifest V3 extension that talks to the Personal Memory **desktop app** over authenticated localhost IPC.

## Prerequisites

1. Run the desktop app (`npm run tauri dev` or a built binary) so the IPC server starts.
2. Desktop listens on **`http://127.0.0.1:17832`** only (loopback).
3. Shared token (must match desktop): `personal-memory-local-dev-token`  
   Header: `X-Memory-Token`

## Load unpacked (Chrome)

1. Open `chrome://extensions`
2. Enable **Developer mode**
3. **Load unpacked** → select this `extension/` directory
4. Pin the extension if you like

## Verify ping

1. Ensure the desktop app is running (look for log: `[ipc] extension IPC listening on http://127.0.0.1:17832`).
2. Click the extension action icon (toolbar).
3. Open the service worker DevTools (extensions page → “service worker” / “Inspect views”) and confirm a log like `ping result { status: 200, data: { ok: true, pong: true, ... } }`.

Wrong token or app not running → non-200 / fetch error.

## Pairing notes

- Port and token are constants in `extension/background.js` and `src-tauri/src/ipc.rs` for local-dev simplicity.
- No cloud endpoints. Do not change the bind address away from `127.0.0.1`.
- Full page URL/selection capture is issue #12.

## Firefox

Not packaged separately yet. Chromium load-unpacked is the supported path for this skeleton.
