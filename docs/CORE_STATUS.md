# DeepSeek-Mobile — core, mobile, and host status

**Updated:** 2026-05-25

## Verification

- `cargo check --workspace --all-targets` — green
- `cargo test --workspace` — green
- Tests:
  - mobile: 102
  - core: 118
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
- Normalized post-edit diagnostics metadata for UI and model context
- Latest diagnostics injection into the next model turn
- Auto snapshot hooks
- Workspace connection model + persistent store
- Public Termux execution request/result contract
- Termux-workspace `exec_shell` now emits native pending request metadata instead of a local placeholder

### Now wired into the engine lifecycle

- `ModelRouter` — auto-selects Flash/Pro per prompt complexity and context size
- `ContextManager` — fits messages within the selected model's context budget
- `auto_commit_and_push` — auto-commits + pushes after successful turns when enabled

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
- Git panel with real status/diff/branch/commit/push/pull actions
- Bottom navigation and cockpit layout

### Important wiring completed

- Saved config drives turns and approval continuations.
- Online PC discovery promotes an active route.
- "Open PC workspace" persists a real `WorkspaceConnection`.
- Future `MobileRuntimeConfig::default()` calls restore the saved active workspace.
- New pairing requests use a generated token instead of an empty token.
- Termux commands now have Rust bridge queue/callback correlation, timeline routing, and automatic queue extraction from pending tool-result metadata.
- Android host integration notes document the native bridge contract boundaries.

### Still partial

- Files diff preview is illustrative, not yet bound to actual patch state.
- Native Android host integration is not complete.
- Termux has bridge contracts and core-to-mobile request queuing, but not the final Android host drain/callback/result-continuation lifecycle.
- Terminal persistence is not complete.

## Android bridge module

### Completed

- Document picker bridge for `ACTION_OPEN_DOCUMENT` and sandbox copies.
- PC gateway NSD/mDNS discovery bridge.
- Termux `RUN_COMMAND` intent builder and result parser.
- Manifest permissions for network discovery and Termux command execution.

### Still planned

- Final Dioxus Android host adapter that drains Rust commands and forwards Kotlin callbacks.
- Manual emulator/device verification against the final host shell.

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

1. Final Android host + Termux callback/result-continuation lifecycle
2. Real project diff surfaces and remote-aware file UI
3. PC-workspace snapshots and terminal persistence
4. Durable tasks, runtime API, MCP/plugins/skills
