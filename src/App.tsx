import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type CaptureSummary = {
  id: string;
  snippet: string;
  captured_at: string;
  source_kind: string | null;
  source_app: string | null;
};

function sourceLabel(item: CaptureSummary): string {
  const parts = [item.source_kind, item.source_app].filter(Boolean);
  return parts.length ? parts.join(" · ") : "unknown source";
}

function App() {
  const [items, setItems] = useState<CaptureSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await invoke<CaptureSummary[]>("list_recent_captures", {
        limit: 50,
      });
      setItems(rows);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("capture-shortcut", () => {
      void refresh();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [refresh]);

  return (
    <main className="inbox">
      <header className="inbox-header">
        <div>
          <p className="eyebrow">Personal Memory</p>
          <h1>Inbox</h1>
          <p className="lede">Recent captures. Capture first; organize later.</p>
        </div>
        <button type="button" onClick={() => void refresh()} disabled={loading}>
          {loading ? "Loading…" : "Refresh"}
        </button>
      </header>

      {error ? <p className="error">{error}</p> : null}

      {!loading && items.length === 0 ? (
        <p className="empty">
          No captures yet. Press Ctrl+Shift+Space anywhere to save something.
        </p>
      ) : (
        <ul className="capture-list">
          {items.map((item) => (
            <li key={item.id} className="capture-item">
              <div className="meta">
                <time dateTime={item.captured_at}>{item.captured_at}</time>
                <span>{sourceLabel(item)}</span>
              </div>
              <p className="snippet">
                {item.snippet.trim() ? item.snippet : "(no text — media/context only)"}
              </p>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}

export default App;
