# Process

## Sources of truth

| Artifact | Role |
|----------|------|
| `_docs/PLAN.md` | Current product direction |
| `_docs/tasks.md` | Project task/backlog overview (index of work) |
| GitHub Issues | Source of truth for **individual** implementation tasks and acceptance criteria |

Do **not** create duplicate task-tracking systems (no second backlog files, shadow todo lists in docs, or parallel issue trackers).

Do **not** copy full task bodies from GitHub Issues back into `tasks.md`. Keep `tasks.md` as an overview/index.

## Roles

- **PM** — grooms a task before anyone implements it, follows `_docs/team/pm.md`.
- **Engineer** — implements one groomed task, follows `_docs/team/software-engineer.md`.
- **QA** — checks the result against the acceptance criteria, follows `_docs/team/qa-engineer.md`.

## Grooming and implementation workflow

1. Select one task from `tasks.md` / GitHub Issues.
2. PM grooms the GitHub issue before implementation.
3. Review the groomed issue.
4. Engineer implements only the groomed scope.
5. Engineer tests the result against the acceptance criteria.
6. Close the issue only when the acceptance criteria are satisfied.

Notes:

- `tasks.md` remains the backlog overview.
- GitHub Issues remain the source of truth for individual tasks.
- Do not create another task-management system.
- Do not duplicate the backlog inside `_docs/`.
- Grooming happens before implementation.
- The PM does not write application code.
- PM rewrites issues using `_docs/task-template.md` (see `_docs/team/pm.md`).

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
