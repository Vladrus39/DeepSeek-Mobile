# DeepSeek-Mobile — Release Notes

## Build 2026-05-25 — Artifacts, Logs & Runtime API

### New

- **Durable task artifacts and logs**: `DurableTaskManager` now supports `add_artifact()`, `append_log()`, and `read_log()` methods. Task log files are stored under `{data_dir}/logs/{task_id}.log` with Unix timestamps. The log path is automatically tracked in the task's `artifact_paths`.
- **PC-host background task logging**: When a PC-host task is started via `RunTask`, its stdout/stderr are piped into a per-task log file in `.deepseek-mobile/tasks/logs/`. The log captures real-time output and is available via the new runtime API.
- **Runtime HTTP API on PC-host**: Two new endpoints:
  - `GET /v1/runtime/tasks` — returns the list of currently running tasks with id, label, kind and started timestamp
  - `GET /v1/runtime/tasks/{task_id}/log` — returns the full task log file content
- **Artifact display in Tasks panel**: Task cards now show artifact count and the first artifact path when present.

### Tests

- 6 new tests for artifact/log methods in `durable_task.rs` (22 total, up from 16)
- Full workspace: 109 mobile / 159 core (6 new) / 2 pc-host tests — all green

### Build

- `cargo check --workspace --all-targets`: clean (only pre-existing dead-code warnings)
- `cargo test --workspace --all-features`: 270 tests pass
- PC-host now depends on `tracing = "0.1"`
