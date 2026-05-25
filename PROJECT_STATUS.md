# DeepSeek-Mobile — current project status

**Updated:** 2026-05-26

## Overall state

DeepSeek-Mobile is in active development with a coherent working core:

- mobile turns use persisted settings instead of hard-coded defaults;
- PC pairing can persist an active workspace connection and normal engine turns can reuse it;
- approvals, snapshots, post-edit diagnostics, PC-host routing and runtime persistence are real code paths, not placeholders;
- latest post-edit diagnostics are stored in the session and injected into the next model turn as model-readable context;
- `apply_patch` now accepts both exact operation batches and standard unified diffs, locally and through PC-gateway routing;
- Termux `exec_shell` is queued through the native bridge and can be continued back into the model with real callback output;
- Git panel actions and engine auto-commit/push lifecycle are wired through real git routes;
- durable task records, queue lifecycle, artifacts/logs, task UI, MCP config registry and skills registry/UI are present;
- PC-host exposes a runtime HTTP task API for listing running tasks and reading per-task logs;
- mobile Tasks panel reconciles active PC-host running tasks through `ListTasks` and can stop PC tasks through `StopTask`;
- Termux workspace selection is available in Settings and activates a persisted Termux runtime workspace;
- core ZIP workspace import/export helpers exist with traversal protection and metadata exclusion;
- Files panel can import a picked project ZIP into the phone workspace and export/share the phone workspace as ZIP;
- mobile UI chrome now exposes live API/PC/workspace state and dynamic badges for approvals, diagnostics, dirty Git state, running tasks and native waits;
- GitHub Actions Rust job installs the Linux GTK/WebKit/pkg-config dependencies required by the Dioxus mobile crate before workspace checks.

## Verified today

| Area | Current state |
|---|---|
| Build | Green |
| Tests | 128 mobile / 166 core / 2 pc-host |
| Mobile settings | Saved config is loaded into live turns and approval continuations |
| GitHub tools | Use token from saved settings first, environment variables second |
| Pairing | Online discovery promotes an active route; “Open PC workspace” persists it |
| Runtime | `MobileRuntimeConfig::default()` loads the saved active workspace when one exists |
| Diagnostics | Rust + TypeScript + Python paths exist; latest diagnostics are re-injected into the next turn |
| Files | Local and active PC workspace browsing use real file data; pending approval diffs are real |
| Tasks | Durable records, queue lifecycle, artifacts/logs, PC-host log capture, mobile task manager UI and PC running-task sync exist |
| Runtime HTTP API | PC-host exposes task list/log endpoints and the mobile UI reconciles running PC tasks; SSE/live event streaming is still pending |
| Termux workspace | Settings selector validates an absolute Termux path and activates a persisted Termux runtime connection |
| Workspace import/export | Files panel exposes project ZIP import/export over the core helpers; final Android host picker/share verification remains pending |
| MCP/skills | Config/manifest registries and mobile UI surfaces exist |
| Android bridge | Document picker, PC discovery, terminal, share and Termux bridge contracts are present |
| Mobile UI | Cockpit screens exist; latest chrome/nav pass compiles; final Android visual verification still pending |

## Implemented but still partial

- Final Android host adapter is still not verified on device/emulator; Rust/Kotlin contracts exist, but production wiring needs a final pass.
- Final visual UI pass is still not verified on device/emulator because the local environment currently lacks Dioxus CLI (`dx`).
- Durable tasks have records/UI, artifacts/logs, PC-host process start/stop/list/log RPCs and manual mobile reconciliation; automatic SSE/live updates remain.
- MCP/skills currently provide registry/config/UI/context surfaces; actual external MCP tool execution must stay behind approval/workspace boundaries when expanded.
- Terminal UI state persists recent sessions/output as closed sessions after restart; live process resurrection is intentionally not claimed.

## Highest-value remaining work

1. Final Dioxus Android host adapter + device/emulator verification for picker, PC discovery, terminal and Termux callbacks.
2. Runtime SSE/live event streaming over the now-stable runtime/task model.
3. Dev-server lifecycle and PC-host autostart/service installer.
4. Release packaging: Android build/release notes, PC-host binary/service notes and troubleshooting docs.

See `docs/PROJECT_AUDIT.md` for the detailed audit and `docs/MASTER_PLAN.md` for the execution backlog.

The intended phone/PC product organization is documented in `docs/PHONE_PC_OPERATING_MODEL.md`.
