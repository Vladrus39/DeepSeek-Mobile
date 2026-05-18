# DeepSeek-Mobile — core, mobile, and host status

**Updated:** 2026-05-18

## Verification

- `cargo check --workspace --all-targets` — green
- `cargo test --workspace` — green
- Tests:
  - mobile: 90
  - core: 114
  - pc-host: 2

## Core crate

### Completed

- DeepSeek streaming client with reasoning deltas
- Session persistence, runtime store, approval continuation
- Tool registry and approval routing
- File, patch, shell, git, web, GitHub, snapshot tools
- Workspace boundary model and PC gateway client
- Saved settings passed into live tool context
- GitHub tools consume persisted token when present
- Local/Termux multi-provider diagnostics aggregation
- Auto snapshot hooks
- Workspace connection model + persistent store

### Present but not yet wired into the main lifecycle

- `ModelRouter`
- `ContextManager`
- `auto_commit_and_push`

## Mobile crate

### Completed surfaces

- Chat/timeline
- Approval cards
- Onboarding/settings
- Files tree + preview
- Snapshots
- Diagnostics
- PC host pairing panel
- Terminal panel
- Git panel surface
- Bottom navigation and cockpit layout

### Important wiring completed today

- Saved config now drives turns and approval continuations.
- Online PC discovery now promotes an active route.
- “Open PC workspace” persists a real `WorkspaceConnection`.
- Future `MobileRuntimeConfig::default()` calls restore the saved active workspace.
- New pairing requests use a generated token instead of an empty token.

### Still partial

- Files diff preview is illustrative, not yet bound to actual patch state.
- Git panel actions are not yet connected to real runtime operations.
- Native Android host integration is not complete.
- Terminal persistence is not complete.

## PC-host crate

### Completed

- HTTP + SSE server
- Auth, policy presets, path traversal protection
- File operations
- Command execution + streaming
- Git operations
- Logs and health details
- Terminal sessions
- Diagnostics:
  - Rust via `cargo check`
  - TypeScript via `tsc`
  - Python via `ruff` / `pyright`

### Still planned

- Dev-server preview lifecycle
- Autostart/service installer
- Persistent terminal restoration

## Highest-priority gaps

1. Final Android host + Termux bridge
2. Diagnostics injection into next model turn
3. Real Git panel wiring and auto-commit lifecycle integration
4. PC-workspace snapshots and remote-aware file UI
5. Durable tasks, runtime API, MCP/plugins/skills
