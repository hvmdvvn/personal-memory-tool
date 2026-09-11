const IPC_BASE = "http://127.0.0.1:17832";
const IPC_TOKEN = "personal-memory-local-dev-token";

async function postIpc(path, body) {
  const res = await fetch(`${IPC_BASE}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Memory-Token": IPC_TOKEN,
    },
    body: JSON.stringify(body),
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

async function pingDesktop() {
  return postIpc("/ping", { type: "ping" });
}

function readPageContext() {
  return {
    url: location.href,
    title: document.title,
    selection: window.getSelection() ? window.getSelection().toString() : "",
  };
}

async function captureActiveTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) {
    throw new Error("no active tab");
  }

  const results = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: readPageContext,
  });
  const payload = results?.[0]?.result;
  if (!payload) {
    throw new Error("could not read page context");
  }

  return postIpc("/capture", {
    url: payload.url ?? tab.url ?? "",
    title: payload.title ?? tab.title ?? "",
    selection: payload.selection ?? "",
  });
}

chrome.action.onClicked.addListener(() => {
  captureActiveTab()
    .then((result) => {
      console.log("[personal-memory] capture result", result);
    })
    .catch((err) => {
      console.error("[personal-memory] capture failed", err);
    });
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type === "ping") {
    pingDesktop()
      .then((result) => sendResponse({ ok: true, result }))
      .catch((err) => sendResponse({ ok: false, error: String(err) }));
    return true;
  }
  if (message?.type === "capture") {
    captureActiveTab()
      .then((result) => sendResponse({ ok: true, result }))
      .catch((err) => sendResponse({ ok: false, error: String(err) }));
    return true;
  }
  return false;
});
