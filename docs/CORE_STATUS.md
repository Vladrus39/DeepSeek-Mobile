# DeepSeek-Mobile — core, mobile, and host status

**Updated:** 2026-05-26

## Verification

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — green.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace` — green.
- Tests:
  - mobile: 140;
  - core: 178;
  - pc-host: 3.
- `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` — green.
- Debug APK install/launch on Samsung `SM_G781B` — smoke test green.

## Core crate

### Completed

- DeepSeek streaming client with reasoning deltas.
- Session persistence, runtime store, approval continuation.
- Tool registry and approval routing.
- File, patch, shell, git, web, GitHub and snapshot tools.
- `apply_patch` supports exact operations and standard unified diffs.
- Workspace boundary model and PC gateway client.
- Saved settings passed into live tool context.
- GitHub tools consume persisted token when present.
- Local/Termux multi-provider diagnostics aggregation.
- Normalized post-edit diagnostics metadata for UI and model context.
- Latest diagnostics injection into the next model turn.
- Auto snapshot hooks, including PC-gateway snapshot RPC routing.
- Workspace connection model + persistent store.
- Durable task records, JSON-backed lifecycle manager, artifacts and logs.
- Runtime task HTTP endpoints in PC-host for task list and task log retrieval.
- Core ZIP workspace import/export helpers with archive path hardening.
- MCP server config registry and local skills manifest registry.
- Public Termux execution request/result contract.
- Termux-workspace `exec_shell` emits native pending request metadata.

### Wired into the engine lifecycle

- `ModelRouter` — auto-selects model per prompt complexity/context size.
- `ContextManager` — fits messages within the selected context budget.
- `auto_commit_and_push` — auto-commits + pushes after successful turns when enabled.
- `continue_termux_result` — resumes a paused turn after Android/Termux callback output arrives.

## Mobile crate

### Completed surfaces

- Chat/timeline.
- Approval cards.
- Onboarding/settings.
- Files tree + preview with real pending diffs.
- Remote-aware file browsing when an active PC workspace is selected.
- Snapshots.
- Diagnostics.
- PC Host pairing panel.
- Terminal panel with persisted UI-state history.
- Git panel with real status/diff/branch/commit/push/pull actions.
- Durable task manager panel.
- PC running-task sync in Tasks panel.
- MCP panel.
- Skills panel.
- Bottom navigation and cockpit layout.
- Termux workspace selector in Settings.
- Project import/export controls in Files panel.
- Android startup screen renders on a physical phone.

### Important wiring completed

- Saved config drives turns and approval continuations.
- Online PC discovery promotes an active route.
- `Open PC workspace` persists a real `WorkspaceConnection`.
- Future `MobileRuntimeConfig::default()` calls restore the saved active workspace.
- New pairing requests use a generated token.
- Termux commands have Rust bridge queue/callback correlation, timeline routing, automatic queue extraction from pending tool-result metadata, and model continuation after callback result.
- Saving the Termux workspace selector activates a persisted Termux runtime connection for future turns.
- Tasks panel reconciles active PC-host running tasks through `ListTasks`, counts them in cockpit badges without double-counting matching local task ids, and can stop them through `StopTask`.
- Android host bridge is packaged into Dioxus builds.

### Still partial

- Full native Android flow verification: picker, import/export/share, Termux callback, PC discovery.
- Terminal UI history persists, but live terminal process resurrection after app restart is not claimed.
- MCP: HTTP + stdio connect/proxy surfaces exist; long-lived stdio session reuse and on-device MCP invoke verification remain.

## Android bridge module

### Completed

- Document picker bridge for `ACTION_OPEN_DOCUMENT` and sandbox copies.
- PC gateway NSD/mDNS discovery bridge.
- Termux `RUN_COMMAND` intent builder and result parser.
- Manifest permissions for network discovery and Termux command execution.
- FileProvider resources for native share.
- `NativeBridge.kt` JNI bridge loading Dioxus `libmain.so`.
- `TermuxResultReceiver` callback delivery.
- Adaptive launcher icon resources.
- Dioxus custom manifest with full config-change handling.

### Still planned

- Manual hardware verification of picker/share project import/export.
- Manual hardware verification of Termux `continue_termux_result`.
- Manual LAN verification of PC Host discovery.

## PC-host crate

### Completed

- HTTP + SSE server.
- Auth, policy presets, path traversal protection.
- File operations.
- Command execution + streaming.
- Git operations.
- Logs and health details.
- Terminal sessions.
- Snapshot create/list/restore RPC path.
- Background task start/stop/list RPC path.
- Diagnostics:
  - Rust via `cargo check`;
  - TypeScript via `tsc`;
  - Python via `ruff` / `pyright`.

### Still planned

- Dev-server preview lifecycle.
- Release bundle with matching host binary.
- Optional autostart/service installer.

## Highest-priority gaps

1. Native Android manual verification checklist.
2. Signed APK/AAB release packaging.
3. PC-host release/service packaging.
4. MCP stdio session reuse and external execution hardening.
