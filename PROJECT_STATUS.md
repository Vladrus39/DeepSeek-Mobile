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
| Tests | 97 mobile / 117 core / 2 pc-host |
| Mobile settings | Saved config is loaded into live turns and approval continuations |
| GitHub tools | Use token from saved settings first, environment variables second |
| Pairing | Online discovery promotes an active route; “Open PC workspace” persists it |
| Runtime | `MobileRuntimeConfig::default()` loads the saved active workspace when one exists |
| Diagnostics | Rust + TypeScript + Python paths exist; latest diagnostics are re-injected into the next turn |
| Android bridge | Document picker, PC discovery, Termux bridge contracts and Termux native command queue extraction are present |

## Implemented but still partial

- Git panel UI exists, but its buttons are still mostly visual and not yet wired to runtime operations.
- The Files panel has a real tree/preview, but its diff preview is still illustrative rather than bound to actual pending patches.
- Terminal sessions exist on PC-host and in UI state, but persistence and full Android runtime wiring are not complete.
- `ModelRouter`, `ContextManager`, and `auto_commit_and_push` exist but are not yet part of the main turn lifecycle.
- Termux now has Rust/Kotlin bridge contracts and core-to-mobile native request queuing, but the Android host drain/callback/result-continuation lifecycle is not yet closed end-to-end.

## Highest-value remaining work

1. Finish final Android host integration: drain queued Termux commands, receive callbacks, and feed the result back as the final `exec_shell` output.
2. Wire Git UI actions and auto-commit/push into real runtime flows.
3. Replace illustrative Files diff preview with real pending/project diffs.
4. Add PC-workspace snapshot support plus terminal persistence.
5. Build durable background tasks, runtime API, then MCP/plugins/skills.

See `docs/PROJECT_AUDIT.md` for the detailed audit and `docs/MASTER_PLAN.md` for the execution backlog.
