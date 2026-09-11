const IPC_BASE = "http://127.0.0.1:17832";
const IPC_TOKEN = "personal-memory-local-dev-token";

async function pingDesktop() {
  const res = await fetch(`${IPC_BASE}/ping`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Memory-Token": IPC_TOKEN,
    },
    body: JSON.stringify({ type: "ping" }),
  });
  const text = await res.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    data = { raw: text };
  }
  return { status: res.status, data };
}

chrome.action.onClicked.addListener(() => {
  pingDesktop()
    .then((result) => {
      console.log("[personal-memory] ping result", result);
    })
    .catch((err) => {
      console.error("[personal-memory] ping failed", err);
    });
});

// Allow other extension pages/tests to request a ping.
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type === "ping") {
    pingDesktop()
      .then((result) => sendResponse({ ok: true, result }))
      .catch((err) => sendResponse({ ok: false, error: String(err) }));
    return true;
  }
  return false;
});
