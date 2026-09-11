# Browser extension (MV3)

Chrome-compatible Manifest V3 extension that talks to the Personal Memory **desktop app** over authenticated localhost IPC.

## Prerequisites

1. Run the desktop app (`npm run tauri dev` or a built binary) so the IPC server starts.
2. Desktop listens on **`http://127.0.0.1:17832`** only (loopback).
3. Shared token (must match desktop): `personal-memory-local-dev-token`  
   Header: `X-Memory-Token`

## Permissions

| Permission | Why |
|------------|-----|
| `activeTab` | Access the tab the user invoked the action on |
| `scripting` | Read `document.title`, `location.href`, and `window.getSelection()` |
| `host_permissions: http://127.0.0.1:17832/*` | Talk to the desktop IPC server |

## Load unpacked (Chrome)

1. Open `chrome://extensions`
2. Enable **Developer mode**
3. **Load unpacked** → select this `extension/` directory
4. Pin the extension if you like

## Manual test: selection → Inbox

1. Start the desktop app; confirm log `[ipc] extension IPC listening on http://127.0.0.1:17832`.
2. Open any web page, **select some text**.
3. Click the extension toolbar action.
4. In the extension service worker console, confirm `capture result` with `status: 200` and an `id`.
5. In the desktop **Inbox**, click **Refresh** (or rely on a later capture-shortcut refresh) and confirm the snippet/source shows the selection with browser URL/title context.

Empty selection still saves a row (empty text) with URL/title.

## Ping (debug)

From the service worker console you can still message `{ type: "ping" }` via `chrome.runtime.sendMessage`, or temporarily call the old ping path used in development.

## Pairing notes

- Port and token are constants in `extension/background.js` and `src-tauri/src/ipc.rs`.
- Endpoints: `POST /ping`, `POST /capture` with JSON `{ url, title, selection }`.
- No cloud endpoints. Do not change the bind address away from `127.0.0.1`.

## Firefox

Not packaged separately yet. Chromium load-unpacked is the supported path for this skeleton.
