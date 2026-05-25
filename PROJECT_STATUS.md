# DeepSeek-Mobile — current project status

**Updated:** 2026-05-25

## Overall state

DeepSeek-Mobile is in active development with a coherent working core:

- mobile turns use persisted settings instead of hard-coded defaults;
- PC pairing can persist an active workspace connection and normal engine turns can reuse it;
- approvals, snapshots, post-edit diagnostics, PC-host routing and runtime persistence are real code paths, not placeholders;
- latest post-edit diagnostics are stored in the session and injected into the next model turn as model-readable context;
- `apply_patch` now accepts both exact operation batches and standard unified diffs, locally and through PC-gateway routing;
- Termux `exec_shell` is queued through the native bridge and can be continued back into the model with real callback output;
- Git panel actions and engine auto-commit/push lifecycle are wired through real git routes;
- durable task records, queue lifecycle, task UI, MCP config registry and skills registry/UI are present;
- GitHub Actions Rust job installs the Linux GTK/WebKit/pkg-config dependencies required by the Dioxus mobile crate before workspace checks.

## Verified today

| Area | Current state |
|---|---|
| Build | Green |
| Tests | 108 mobile / 152 core / 2 pc-host |
| Mobile settings | Saved config is loaded into live turns and approval continuations |
| GitHub tools | Use token from saved settings first, environment variables second |
| Pairing | Online discovery promotes an active route; “Open PC workspace” persists it |
| Runtime | `MobileRuntimeConfig::default()` loads the saved active workspace when one exists |
| Diagnostics | Rust + TypeScript + Python paths exist; latest diagnostics are re-injected into the next turn |
| Files | Local and active PC workspace browsing use real file data; pending approval diffs are real |
| Tasks | Durable records, queue lifecycle and mobile task manager UI exist |
| MCP/skills | Config/manifest registries and mobile UI surfaces exist |
| Android bridge | Document picker, PC discovery, terminal, share and Termux bridge contracts are present |

## Implemented but still partial

- Final Android host adapter is still not verified on device/emulator; Rust/Kotlin contracts exist, but production wiring needs a final pass.
- Durable tasks have records/UI and PC-host process start/stop/list RPCs; artifacts/logs per task and tighter PC-running-task synchronization remain.
- MCP/skills currently provide registry/config/UI/context surfaces; actual external MCP tool execution must stay behind approval/workspace boundaries when expanded.
- Terminal UI state persists recent sessions/output as closed sessions after restart; live process resurrection is intentionally not claimed.

## Highest-value remaining work

1. Final Dioxus Android host adapter + device/emulator verification for picker, PC discovery, terminal and Termux callbacks.
2. Termux workspace selector and Android import/export completion.
3. Runtime HTTP/SSE API over the now-stable runtime/task model.
4. Durable task artifacts/logs and PC-running-task synchronization.
5. Release packaging: Android build/release notes, PC-host binary/service notes and troubleshooting docs.

See `docs/PROJECT_AUDIT.md` for the detailed audit and `docs/MASTER_PLAN.md` for the execution backlog.
