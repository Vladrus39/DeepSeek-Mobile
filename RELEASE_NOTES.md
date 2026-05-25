# DeepSeek-Mobile — Release Notes

## Build 2026-05-25 — Artifacts, Logs, Runtime API, Termux Workspace & Workspace IO

### New

- **Durable task artifacts and logs**: `DurableTaskManager` supports `add_artifact()`, `append_log()`, and `read_log()`. Task log files are stored under `{data_dir}/logs/{task_id}.log` with Unix timestamps, and the log path is tracked in `artifact_paths`.
- **PC-host background task logging**: `RunTask` pipes stdout/stderr into per-task logs under `.deepseek-mobile/tasks/logs/`.
- **Runtime HTTP task API on PC-host**:
  - `GET /v1/runtime/tasks` — returns currently running tasks with id, label, kind and started timestamp.
  - `GET /v1/runtime/tasks/{task_id}/log` — returns the task log content.
- **Artifact display in Tasks panel**: task cards show artifact count and the first artifact path when present.
- **Termux workspace selector**: Settings now has a Termux workspace section. Saving a valid absolute Termux path persists `termux_workspace.json` and activates a `WorkspaceConnection` so future mobile turns run against the Termux backend.
- **Termux path hardening**: the selector rejects empty paths, relative paths, parent-directory traversal, Windows-style paths/backslashes and invalid saved configs on reload.
- **Core workspace ZIP import/export helpers**: `workspace_io` can import/export project ZIPs, rejects unsafe archive paths, emits portable ZIP entry names and excludes `.deepseek-mobile` metadata from exports.

### Tests

- Full workspace: 120 mobile / 166 core / 2 pc-host tests — all green.
- Added Termux workspace state tests for activation, strict path validation and invalid saved-config revalidation.
- Added workspace import/export tests for ZIP traversal rejection, Windows/absolute path rejection, metadata exclusion and export/reimport roundtrip.
- Durable task artifact/log coverage is included in the current core count.

### Build

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets`: green, with existing dead-code warnings in mobile test-only/native bridge surfaces.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace`: 288 tests pass.
- PC-host depends on `tracing = "0.1"` for task log capture.

### Still pending

- Runtime SSE/live event streaming and richer task reconciliation.
- Final Android/Dioxus host adapter and device/emulator verification.
- Android project import/export UI flow over the new core ZIP helpers.
