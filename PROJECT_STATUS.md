# DeepSeek-Mobile — current project status

**Updated:** 2026-05-25

## Overall state

DeepSeek-Mobile is in active development but now has a coherent working core:

- mobile turns use persisted settings instead of hard-coded defaults;
- PC pairing can persist an active workspace connection and normal engine turns can reuse it;
- approvals, snapshots, post-edit diagnostics, PC-host routing and runtime persistence are real code paths, not placeholders;
- latest post-edit diagnostics are stored in the session and injected into the next model turn as model-readable context;
- the Android bridge module now includes a Termux `RUN_COMMAND` adapter contract in addition to document picker and PC discovery adapters;
- approved `exec_shell` calls in a Termux workspace now emit a structured native `TermuxExecRequest`, and the mobile layer queues that request into `NativeBridgeState`;
- local verification is green: `cargo check --workspace --all-targets` and `cargo test --workspace`;
- GitHub Actions Rust job now installs the Linux GTK/WebKit/pkg-config dependencies required by the Dioxus mobile crate before running workspace checks.

## Verified today

| Area | Current state |
|---|---|
| Build | Green |
| Tests | 102 mobile / 118 core / 2 pc-host |
| Mobile settings | Saved config is loaded into live turns and approval continuations |
| GitHub tools | Use token from saved settings first, environment variables second |
| Pairing | Online discovery promotes an active route; “Open PC workspace” persists it |
| Runtime | `MobileRuntimeConfig::default()` loads the saved active workspace when one exists |
| Diagnostics | Rust + TypeScript + Python paths exist; latest diagnostics are re-injected into the next turn |
| Android bridge | Document picker, PC discovery, Termux bridge contracts and Termux native command queue extraction are present |

## Implemented but still partial

- Git panel actions now run real status/diff/branch/commit/push/pull operations through the existing tool route; auto-commit/push is now part of the engine lifecycle after successful turns when enabled.
- The Files panel diff preview now shows real diffs computed from pending approval cards (write_file/edit_file). When no pending change matches the selected file, it shows "No pending changes" instead of a fake hook.
- Terminal sessions exist on PC-host and in UI state, but persistence and full Android runtime wiring are not complete.
- `ModelRouter`, `ContextManager`, and `auto_commit_and_push` are now wired into the engine lifecycle.
- Termux callback/result-continuation is now closed end-to-end: when the Android Termux bridge returns real command output, the engine injects it into the session and re-queries the model so it can respond to actual results. The Rust-side turn lifecycle handles `WaitingForTermuxResult` status and `continue_termux_result` continuation.

## Highest-value remaining work

1. ~~Make file browsing remote-aware when PC workspace is active.~~ ✅ Done (2026-05-25)
2. ~~Add PC-workspace snapshot support plus terminal persistence.~~ ✅ Done (2026-05-25): snapshot_create/list/restore now route through PC gateway; terminal sessions persist to disk and restore on launch.
3. ~~Build durable background tasks, runtime API, then MCP/plugins/skills.~~ Background task infrastructure ✅ (2026-05-25): PC host can spawn/stop/list background tasks via `run_task`/`stop_task`/`list_tasks` gateway RPC; tool routing wired for `detect_tasks`, `task_run`, `task_stop`, `task_list`. Durable task records with queue lifecycle and JSON persistence ✅ (2026-05-25): `DurableTaskManager` in core with full CRUD, status transitions, and 16 tests. Remaining: runtime API + mobile task manager UI + MCP/plugins/skills.

See `docs/PROJECT_AUDIT.md` for the detailed audit and `docs/MASTER_PLAN.md` for the execution backlog.