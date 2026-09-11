# Process

## Sources of truth

| Artifact | Role |
|----------|------|
| `_docs/PLAN.md` | Current product direction |
| `_docs/tasks.md` | Project task/backlog overview (index of work) |
| GitHub Issues | Source of truth for **individual** implementation tasks and acceptance criteria |

Do **not** create duplicate task-tracking systems (no second backlog files, shadow todo lists in docs, or parallel issue trackers).

Do **not** copy full task bodies from GitHub Issues back into `tasks.md`. Keep `tasks.md` as an overview/index.

## Working on a task

1. Identify the single GitHub issue to implement (from the user request or `_docs/tasks.md`).
2. Read the issue before writing code. Treat **Goal** and **Description** as acceptance criteria.
3. Work on **one task at a time**.
4. Keep changes focused on that issue; do not start unrelated work unless explicitly requested.
5. Test changes before considering the issue complete (see `_docs/testing-guidelines.md`).
6. Update relevant documentation when project decisions change (plan, architecture, design, testing, or `AGENTS.md` command list when real commands appear).
7. Commit completed work regularly with messages that reflect why the change was made.

## After scaffolding exists

Record how to build, run, and test in the README and keep the Commands section in `AGENTS.md` aligned with commands that actually exist.
