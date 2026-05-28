---
name: project-rules-discipline
description: Follow and extend project rules—AGENTS.md, docs/CURRENT_STATE.md, user coding standards.
---

## Purpose

Keep agent behavior aligned with repository conventions (like Cursor project rules).

## Always read first

- `docs/CURRENT_STATE.md` — factual checkpoint.
- `docs/TROUBLESHOOTING.md` — known device issues.
- `README.md` — build/install commands.

## When editing

- Match existing naming, error handling depth, and module layout.
- Minimize diff scope; no drive-by refactors.
- Update docs when behavior or E2E status changes.

## Phone-first priority

1. Termux agent + tools
2. MCP / ZIP / UI flows on device
3. PC Host pairing — last phase unless user asks
