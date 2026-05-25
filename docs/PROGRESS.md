# DeepSeek-Mobile — active progress

**Current session:** 2026-05-25

## Completed in the latest tranche

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
- Cleaned fresh warning/noise in touched code (`tool_loop`, Files panel/state, task panel CSS values, `main.rs` unused bindings).
- Updated local documentation to reflect what was already completed while I was away: PC snapshots, remote-aware Files, durable tasks, task UI, MCP/skills registry/UI, terminal UI-state persistence, and Termux continuation.

## Verification

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — passed
- Targeted tests passed:
  - `deepseek-mobile-core tools::patch::tests`
  - `deepseek-mobile-core tool_execution::tests::remote_patch_operation_deserializes_normalized_unified_diff`
  - `deepseek-mobile terminal_state::tests`
- Full workspace test target after this tranche:
  - mobile: 108
  - core: 152
  - pc-host: 2

## Current focus

The remaining product gaps are concentrated around production integration and release readiness:

1. final Dioxus Android host adapter and device/emulator verification;
2. Termux workspace selector and Android import/export completion;
3. runtime HTTP/SSE API over the stable runtime/task model;
4. durable task artifacts/logs and PC-running-task reconciliation;
5. dev-server lifecycle, PC-host service/autostart and release/troubleshooting docs.

## Notes from the audit

- PC-host already contains Rust, TypeScript, and Python diagnostics implementations.
- Diagnostics are both UI-visible and model-readable on the following turn.
- The Git UI runs real status/diff/branch/commit/push/pull actions; engine auto-commit is also wired behind config.
- Files browsing is now active-PC-aware, but deeper editor/project-diff UX can still be improved later.
- MCP/skills currently means registry/config/UI/context surfaces, not unrestricted external tool execution.
