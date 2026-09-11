# Design system

Initial UI principles for the personal memory tool. Derived from `_docs/PLAN.md`.  
**No detailed colors, typography tokens, or component inventory yet**—those should be chosen when UI work begins.

## Product UX goals

- **Extremely fast capture** — Saving something useful should feel instant; the primary path is one global shortcut, not a multi-step form.
- **Minimal friction** — Do not force notebook, folder, type, tags, or metadata choices at capture time. Capture first; organize later.
- **Local-first experience** — The UI should feel like a private desktop tool. Avoid cloud-account or sync-first patterns in the default experience.
- **Simple, clean UI** — The system is powerful (search, connections, resurfacing, assistant); the interface should stay calm and uncluttered.
- **Clear information hierarchy** — One job per view when possible. Lead with what the user needs next (recent captures, search results, resurfaced items, answer + citations), not chrome.

## Planned surfaces (from the plan)

Keep each area focused when implemented:

- **Inbox** — Recently captured items
- **Memory** — Organized personal knowledge
- **Search** — Exact and natural-language retrieval
- **Connections** — Related items
- **Today** — Resurfaced memories
- **AI Assistant** — Chat over the user’s stored knowledge

## Interaction guidance

- Prefer progressive disclosure: advanced organization and AI detail after capture, not before.
- Show originals clearly; present AI labels/metadata as secondary, never as a replacement for the raw capture.
- Empty and error states should be plain and actionable (e.g. Ollama unavailable → exact search still works when that split exists).
- Density should favor scannable lists and short snippets over dashboards of competing widgets.

## Out of scope for this document (for now)

- Color palette, type scale, spacing tokens
- Component library choices
- Motion/brand illustration specs
