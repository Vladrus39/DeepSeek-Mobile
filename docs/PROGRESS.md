# DeepSeek-Mobile — active progress

**Current session:** 2026-05-26 (docs + Android toolchain isolation)

## Completed in the latest tranche (2026-05-26)

- Closed the Android host Rust/Kotlin/JNI loop:
  - `android_host::apply_host_callback_json` handles picker picked, PC discovery, Termux;
  - `dev.dioxus.main.MainActivity` (`WryActivity` + host coordinator poll);
  - JNI `NativeBridge` + `native_host_runtime` sync in the Dioxus UI loop.
- Added project-local Android toolchain under `tools/android/` (~255 MB SDK slice copied from system install; no extra download).
- Documented exact online download budget in `tools/android/DOWNLOAD_BUDGET.md` (~1.0–1.2 GB for APK on a real phone).
- Core/mobile: encrypted `config_store`, MCP HTTP/stdio client, MCP proxy tools in engine, desktop native host execution, PC-host install scripts.
- Full workspace tests green: **140 mobile / 178 core / 3 pc-host**.

## Completed in earlier tranches (same sprint)

- Audited the user's latest local/GitHub work after `artifacts-log-capture-runtime-API` and kept it instead of redoing it.
- Completed the Termux workspace selector slice:
  - Settings now validates and saves an absolute Termux path;
  - saving activates a persisted Termux `WorkspaceConnection` for future turns;
  - invalid saved Termux configs are revalidated on load instead of being trusted.
- Hardened the new core workspace import/export helper:
  - ZIP import rejects `..`, absolute paths, Windows drive prefixes and backslash traversal;
  - ZIP export emits portable `/` entry names and excludes `.deepseek-mobile` metadata;
  - missing workspace roots now fail clearly before export.
- Added Files panel project import/export UI over the core ZIP helpers:
  - Import ZIP queues `DocumentPickerRequest::project_import()` and imports the returned local archive copy into the phone workspace;
  - Export ZIP writes `.deepseek-mobile/exports/deepseek-mobile-project-*.zip` and queues native share;
  - Android picker callbacks now route by `DocumentPickerPurpose`, so project archives no longer become chat attachments.
- Added PC running-task reconciliation in the mobile Tasks panel:
  - the panel syncs active PC-host tasks through `PcGatewayClient::list_tasks()`;
  - running PC tasks are displayed separately from local durable records;
  - the cockpit task badge counts local and PC active work without double-counting matching ids;
  - active PC tasks can be stopped through `PcGatewayClient::stop_task()`.
- Refreshed local docs to show that durable task artifacts/logs, PC-host runtime task HTTP endpoints, project import/export UI, PC task SSE subscription and Android host coordinator are implemented; JNI/device verification and release packaging remain open.
- Audited the local `main` after four new local commits (`PhaseD2` → `PhaseG`) and confirmed the working tree was clean before continuing.
- Verified the new local state before edits: `cargo check` and `cargo test` were green.
- Added unified-diff compatibility to `apply_patch` while preserving the existing operation-batch API.
  - Local `ApplyPatchTool` now accepts `operations`, `unified_diff`, or `patch`.
  - PC-gateway `apply_patch` routing normalizes unified diffs into the same safe operation model before remote execution.
  - Added tests for modify/create unified diffs, rollback on later hunk failure, and remote operation deserialization.
- Hardened the new Files panel PC path:
  - the panel now passes the actual active `WorkspaceConnection.workspace_id` into PC-gateway file browsing instead of reusing the display root;
  - backend switches between local and PC trigger a refresh;
  - stale unused refresh state was removed.
- Fixed terminal persistence behavior:
  - saved terminal UI state is loaded only once instead of on every render;
  - parent directories are created before saving;
  - restored sessions are intentionally marked closed after restart;
  - truncation markers now report the real number of dropped output lines.
- Fixed Linux CI-only nondeterminism in skills discovery by sorting discovered `SKILL.md` paths before duplicate-name resolution.
- Cleaned fresh warning/noise in touched code (`tool_loop`, Files panel/state, task panel CSS values, `main.rs` unused bindings).
- Updated local documentation to reflect what was already completed while I was away: PC snapshots, remote-aware Files, durable tasks, task UI, MCP/skills registry/UI, terminal UI-state persistence, and Termux continuation.
- Added `docs/PHONE_PC_OPERATING_MODEL.md` to lock the intended product architecture: Android app as the primary UI/orchestrator, Termux and PC Host as execution backends, optional DeepSeek-TUI compatibility without depending on TUI internals, and a target PC bootstrap flow.
- Improved PC pairing launcher bootstrap: Windows/macOS/Linux scripts now prefer a bundled `deepseek-pc-host` binary next to the pairing files, fall back to `PATH`, and emit a clear install/release-package instruction when missing.
- Updated GitHub Actions checkout steps to `actions/checkout@v6` so CI runs on the Node.js 24 action runtime instead of the deprecated Node.js 20 runtime.
- Added a mobile UI chrome pass: live API/PC chips, active workspace summary, dynamic drawer/bottom-nav badges, scrollable bottom navigation, and removal of an invalid `space_around` CSS value.
- Added `docs/UI_STATUS_AND_VERIFICATION.md` to separate what is already implemented from what still needs real Android visual/device verification.

## Verification

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — passed
- Targeted tests passed:
  - `deepseek-mobile-core tools::patch::tests`
  - `deepseek-mobile-core tool_execution::tests::remote_patch_operation_deserializes_normalized_unified_diff`
  - `deepseek-mobile terminal_state::tests`
  - `deepseek-mobile termux_state::tests`
  - `deepseek-mobile-core workspace_io::tests`
  - `deepseek-mobile project_transfer_state::tests`
  - `deepseek-mobile tasks_state::tests`
- Full workspace test (latest):
  - mobile: 137
  - core: 170
  - pc-host: 3

## Current focus

1. Device/emulator verification: `dx build android` after NDK + `dioxus-cli` install (see `tools/android/DOWNLOAD_BUDGET.md`).
2. Signed APK / release packaging and PC-host autostart on real machines.
3. Long-lived MCP stdio session reuse and hardware verification of MCP proxy tools.

## Notes from the audit

- PC-host already contains Rust, TypeScript, and Python diagnostics implementations.
- Diagnostics are both UI-visible and model-readable on the following turn.
- The Git UI runs real status/diff/branch/commit/push/pull actions; engine auto-commit is also wired behind config.
- Files browsing is now active-PC-aware, but deeper editor/project-diff UX can still be improved later.
- MCP/skills currently means registry/config/UI/context surfaces, not unrestricted external tool execution.
