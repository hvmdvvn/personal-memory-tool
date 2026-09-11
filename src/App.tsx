import { FormEvent, useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type View = "inbox" | "search";

type CaptureSummary = {
  id: string;
  snippet: string;
  captured_at: string;
  source_kind: string | null;
  source_app: string | null;
};

type UnifiedHit = {
  id: string;
  snippet: string;
  captured_at: string;
  source_kind: string | null;
  source_app: string | null;
  match_reason: string;
  score: number | null;
  content_type: string | null;
};

const CONTENT_TYPES = [
  "any",
  "book_quote",
  "idea",
  "note",
  "article",
  "web_link",
  "video_post",
  "conversation",
  "task",
  "general",
] as const;

function sourceLabel(item: {
  source_kind: string | null;
  source_app: string | null;
}): string {
  const parts = [item.source_kind, item.source_app].filter(Boolean);
  return parts.length ? parts.join(" · ") : "unknown source";
}

function App() {
  const [view, setView] = useState<View>("inbox");
  const [items, setItems] = useState<CaptureSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [query, setQuery] = useState("");
  const [contentType, setContentType] = useState<string>("any");
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");
  const [hits, setHits] = useState<UnifiedHit[]>([]);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searched, setSearched] = useState(false);

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

  const runSearch = useCallback(
    async (event?: FormEvent) => {
      event?.preventDefault();
      const q = query.trim();
      if (!q) {
        setHits([]);
        setSearched(true);
        setSearchError(null);
        return;
      }
      setSearchLoading(true);
      setSearchError(null);
      setSearched(true);
      try {
        const rows = await invoke<UnifiedHit[]>("search_unified", {
          query: q,
          limit: 50,
          contentType: contentType === "any" ? null : contentType,
          fromCapturedAt: fromDate ? `${fromDate}T00:00:00Z` : null,
          toCapturedAt: toDate ? `${toDate}T23:59:59Z` : null,
        });
        setHits(rows);
      } catch (e) {
        setSearchError(String(e));
        setHits([]);
      } finally {
        setSearchLoading(false);
      }
    },
    [query, contentType, fromDate, toDate],
  );

  return (
    <main className="app-shell">
      <nav className="top-nav" aria-label="Primary">
        <button
          type="button"
          className={view === "inbox" ? "nav-active" : undefined}
          onClick={() => setView("inbox")}
        >
          Inbox
        </button>
        <button
          type="button"
          className={view === "search" ? "nav-active" : undefined}
          onClick={() => setView("search")}
        >
          Search
        </button>
      </nav>

      {view === "inbox" ? (
        <section className="inbox">
          <header className="inbox-header">
            <div>
              <p className="eyebrow">Personal Memory</p>
              <h1>Inbox</h1>
              <p className="lede">
                Recent captures. Capture first; organize later.
              </p>
            </div>
            <button
              type="button"
              onClick={() => void refresh()}
              disabled={loading}
            >
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
                    {item.snippet.trim()
                      ? item.snippet
                      : "(no text — media/context only)"}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </section>
      ) : (
        <section className="inbox search-view">
          <header className="inbox-header">
            <div>
              <p className="eyebrow">Personal Memory</p>
              <h1>Search</h1>
              <p className="lede">
                Exact and semantic matches. Filters apply server-side.
              </p>
            </div>
          </header>

          <form className="search-form" onSubmit={(e) => void runSearch(e)}>
            <label className="field">
              <span>Query</span>
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Find a quote, idea, or note…"
                autoFocus
              />
            </label>
            <div className="filters">
              <label className="field">
                <span>Type</span>
                <select
                  value={contentType}
                  onChange={(e) => setContentType(e.target.value)}
                >
                  {CONTENT_TYPES.map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>From</span>
                <input
                  type="date"
                  value={fromDate}
                  onChange={(e) => setFromDate(e.target.value)}
                />
              </label>
              <label className="field">
                <span>To</span>
                <input
                  type="date"
                  value={toDate}
                  onChange={(e) => setToDate(e.target.value)}
                />
              </label>
            </div>
            <button type="submit" disabled={searchLoading}>
              {searchLoading ? "Searching…" : "Search"}
            </button>
          </form>

          {searchError ? <p className="error">{searchError}</p> : null}

          {searched && !searchLoading && !searchError && hits.length === 0 ? (
            <p className="empty">
              {query.trim()
                ? "No matches. Try different words or clear filters."
                : "Enter a query to search."}
            </p>
          ) : null}

          {hits.length > 0 ? (
            <ul className="capture-list">
              {hits.map((hit) => (
                <li key={hit.id} className="capture-item">
                  <div className="meta">
                    <time dateTime={hit.captured_at}>{hit.captured_at}</time>
                    <span>{hit.match_reason}</span>
                    <span>{hit.content_type ?? "untyped"}</span>
                    {hit.score != null ? (
                      <span>score {hit.score.toFixed(3)}</span>
                    ) : null}
                  </div>
                  <p className="snippet">
                    {hit.snippet.trim() ? hit.snippet : "(empty snippet)"}
                  </p>
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      )}
    </main>
  );
}

export default App;
