import { FormEvent, useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type View = "inbox" | "search" | "assistant" | "connections" | "today";

type CaptureSummary = {
  id: string;
  snippet: string;
  captured_at: string;
  source_kind: string | null;
  source_app: string | null;
};

type ResurfaceItem = {
  id: string;
  snippet: string;
  captured_at: string;
  source_kind: string | null;
  source_app: string | null;
  content_type: string | null;
  short_description: string | null;
  rank: number;
};

type TodayResurfacing = {
  day: string;
  items: ResurfaceItem[];
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

type RelatedHit = {
  id: string;
  snippet: string;
  captured_at: string;
  source_kind: string | null;
  source_app: string | null;
  score: number;
  metadata_boost: boolean;
};

type QaAnswer = {
  answer: string;
  citation_ids: string[];
  model: string;
};

type ChatMessage =
  | { role: "user"; text: string }
  | { role: "assistant"; text: string; citation_ids: string[] };

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

const NAV: { id: View; label: string }[] = [
  { id: "inbox", label: "Inbox" },
  { id: "today", label: "Today" },
  { id: "search", label: "Search" },
  { id: "assistant", label: "Assistant" },
  { id: "connections", label: "Connections" },
];

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

  const [chat, setChat] = useState<ChatMessage[]>([]);
  const [question, setQuestion] = useState("");
  const [askLoading, setAskLoading] = useState(false);
  const [askError, setAskError] = useState<string | null>(null);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [related, setRelated] = useState<RelatedHit[]>([]);
  const [relatedLoading, setRelatedLoading] = useState(false);
  const [relatedError, setRelatedError] = useState<string | null>(null);
  const [relatedEmptyHint, setRelatedEmptyHint] = useState<string | null>(null);

  const [today, setToday] = useState<TodayResurfacing | null>(null);
  const [todayLoading, setTodayLoading] = useState(false);
  const [todayError, setTodayError] = useState<string | null>(null);

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

  const ask = useCallback(
    async (event?: FormEvent) => {
      event?.preventDefault();
      const q = question.trim();
      if (!q || askLoading) return;
      setAskError(null);
      setAskLoading(true);
      setChat((prev) => [...prev, { role: "user", text: q }]);
      setQuestion("");
      try {
        const ans = await invoke<QaAnswer>("ask_memories", {
          question: q,
          topK: 5,
        });
        setChat((prev) => [
          ...prev,
          {
            role: "assistant",
            text: ans.answer,
            citation_ids: ans.citation_ids,
          },
        ]);
      } catch (e) {
        setAskError(String(e));
      } finally {
        setAskLoading(false);
      }
    },
    [question, askLoading],
  );

  const loadRelated = useCallback(async (id: string) => {
    setSelectedId(id);
    setRelatedLoading(true);
    setRelatedError(null);
    setRelatedEmptyHint(null);
    setRelated([]);
    try {
      const rows = await invoke<RelatedHit[]>("related_to", {
        captureId: id,
        limit: 10,
      });
      setRelated(rows);
      if (rows.length === 0) {
        setRelatedEmptyHint(
          "No related items (capture may lack an embedding—run embed_capture first).",
        );
      }
    } catch (e) {
      setRelatedError(String(e));
    } finally {
      setRelatedLoading(false);
    }
  }, []);

  const loadToday = useCallback(async () => {
    setTodayLoading(true);
    setTodayError(null);
    try {
      const set = await invoke<TodayResurfacing>("ensure_today_resurfacing");
      setToday(set);
    } catch (e) {
      setTodayError(String(e));
    } finally {
      setTodayLoading(false);
    }
  }, []);

  useEffect(() => {
    if (view === "today") {
      void loadToday();
    }
  }, [view, loadToday]);

  return (
    <main className="app-shell">
      <nav className="top-nav" aria-label="Primary">
        {NAV.map((n) => (
          <button
            key={n.id}
            type="button"
            className={view === n.id ? "nav-active" : undefined}
            onClick={() => setView(n.id)}
          >
            {n.label}
          </button>
        ))}
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
      ) : null}

      {view === "search" ? (
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
      ) : null}

      {view === "assistant" ? (
        <section className="inbox">
          <header className="inbox-header">
            <div>
              <p className="eyebrow">Personal Memory</p>
              <h1>Assistant</h1>
              <p className="lede">
                Ask questions answered only from your local captures.
              </p>
            </div>
          </header>

          {chat.length === 0 ? (
            <p className="empty">
              Ask about something you saved—citations will list the memory ids
              used.
            </p>
          ) : (
            <ul className="chat-list">
              {chat.map((msg, i) => (
                <li
                  key={`${msg.role}-${i}`}
                  className={`chat-bubble chat-${msg.role}`}
                >
                  <p className="snippet">{msg.text}</p>
                  {msg.role === "assistant" && msg.citation_ids.length > 0 ? (
                    <p className="citations">
                      Citations: {msg.citation_ids.join(", ")}
                    </p>
                  ) : null}
                </li>
              ))}
            </ul>
          )}

          {askError ? <p className="error">{askError}</p> : null}

          <form className="search-form" onSubmit={(e) => void ask(e)}>
            <label className="field">
              <span>Question</span>
              <input
                type="text"
                value={question}
                onChange={(e) => setQuestion(e.target.value)}
                placeholder="What did I save about…?"
                disabled={askLoading}
              />
            </label>
            <button type="submit" disabled={askLoading || !question.trim()}>
              {askLoading ? "Thinking…" : "Ask"}
            </button>
          </form>
        </section>
      ) : null}

      {view === "connections" ? (
        <section className="inbox">
          <header className="inbox-header">
            <div>
              <p className="eyebrow">Personal Memory</p>
              <h1>Connections</h1>
              <p className="lede">
                Pick a capture to see embedding neighbors.
              </p>
            </div>
            <button type="button" onClick={() => void refresh()} disabled={loading}>
              {loading ? "Loading…" : "Refresh"}
            </button>
          </header>

          {error ? <p className="error">{error}</p> : null}

          {items.length === 0 && !loading ? (
            <p className="empty">No captures to connect yet.</p>
          ) : (
            <ul className="capture-list">
              {items.map((item) => (
                <li key={item.id} className="capture-item">
                  <button
                    type="button"
                    className={
                      selectedId === item.id ? "pick-active pick" : "pick"
                    }
                    onClick={() => void loadRelated(item.id)}
                  >
                    <div className="meta">
                      <time dateTime={item.captured_at}>{item.captured_at}</time>
                      <span>{sourceLabel(item)}</span>
                    </div>
                    <p className="snippet">
                      {item.snippet.trim() ? item.snippet : "(empty)"}
                    </p>
                  </button>
                </li>
              ))}
            </ul>
          )}

          {selectedId ? (
            <div className="related-panel">
              <h2>Related to {selectedId}</h2>
              {relatedLoading ? <p className="empty">Loading…</p> : null}
              {relatedError ? <p className="error">{relatedError}</p> : null}
              {relatedEmptyHint && !relatedLoading ? (
                <p className="empty">{relatedEmptyHint}</p>
              ) : null}
              {related.length > 0 ? (
                <ul className="capture-list">
                  {related.map((hit) => (
                    <li key={hit.id} className="capture-item">
                      <div className="meta">
                        <time dateTime={hit.captured_at}>{hit.captured_at}</time>
                        <span>score {hit.score.toFixed(3)}</span>
                        {hit.metadata_boost ? <span>metadata boost</span> : null}
                      </div>
                      <p className="snippet">{hit.snippet}</p>
                    </li>
                  ))}
                </ul>
              ) : null}
            </div>
          ) : null}
        </section>
      ) : null}

      {view === "today" ? (
        <section className="inbox">
          <header className="inbox-header">
            <div>
              <p className="eyebrow">Personal Memory</p>
              <h1>Today</h1>
              <p className="lede">
                Older memories to revisit. Picked once per UTC day when you open
                this view.
              </p>
            </div>
            <button
              type="button"
              onClick={() => void loadToday()}
              disabled={todayLoading}
            >
              {todayLoading ? "Loading…" : "Refresh"}
            </button>
          </header>

          {todayError ? <p className="error">{todayError}</p> : null}

          {!todayLoading && today && today.items.length === 0 ? (
            <p className="empty">
              Nothing to resurface yet. Need captures older than 7 days.
            </p>
          ) : null}

          {today && today.items.length > 0 ? (
            <>
              <p className="meta">Day {today.day}</p>
              <ul className="capture-list">
                {today.items.map((item) => (
                  <li key={item.id} className="capture-item">
                    <div className="meta">
                      <time dateTime={item.captured_at}>{item.captured_at}</time>
                      <span>{item.content_type ?? "untyped"}</span>
                    </div>
                    {item.short_description ? (
                      <p className="snippet">{item.short_description}</p>
                    ) : (
                      <p className="snippet">
                        {item.snippet.trim() ? item.snippet : "(empty)"}
                      </p>
                    )}
                  </li>
                ))}
              </ul>
            </>
          ) : null}
        </section>
      ) : null}
    </main>
  );
}

export default App;
