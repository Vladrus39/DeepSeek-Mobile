# DeepSeek-Mobile — active progress

**Current session:** 2026-05-18

## Completed in the latest tranche

- Persisted settings are now used by normal turns and approval continuations instead of `Config::default()`.
- `ToolContext` now carries `external_access` and a saved GitHub token; GitHub tools prefer the persisted token over environment variables.
- Fixed multi-provider diagnostics aggregation so `Completed`, `Failed`, `Unavailable`, and `NotApplicable` states stay truthful.
- Stabilized `auto_commit` tests and fixed Unicode-safe commit-message length checking.
- Closed the pairing/runtime gap:
  - online discovery now promotes an active route;
  - pairing creates a real `WorkspaceConnection`;
  - “Open PC workspace” persists the selected route in `workspace_connections.json`;
  - `MobileRuntimeConfig::default()` reloads the active connection on future turns.
- Replaced the insecure empty default pairing token with a generated UUID token.

## Verification

- `cargo check --workspace --all-targets` — passed
- `cargo test --workspace` — passed
- Test totals after this tranche:
  - mobile: 90
  - core: 114
  - pc-host: 2

## Current focus

The next real product gaps are no longer basic compile/runtime wiring. They are:

1. final Android host integration plus Termux bridge;
2. real Git panel action wiring and auto-commit lifecycle integration;
3. diagnostics injection into the next model turn;
4. real diff surfaces for project files instead of preview scaffolding;
5. terminal persistence and PC-workspace snapshot support.

## Notes from the audit

- PC-host already contains Rust, TypeScript, and Python diagnostics implementations.
- The Files panel is useful but still local-preview oriented; it is not yet a remote-aware workspace browser.
- The Git UI is a surface, not yet a fully connected workflow.
- `ModelRouter`, `ContextManager`, and `auto_commit_and_push` are available building blocks, not yet active orchestration features.
