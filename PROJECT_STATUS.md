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
- mobile Tasks panel now subscribes to live PC-host task events via SSE (`stream_task_events`) and reconciles running tasks in real-time;
- Termux workspace selection is available in Settings and activates a persisted Termux runtime workspace;
- core ZIP workspace import/export helpers exist with traversal protection and metadata exclusion;
- Files panel can import a picked project ZIP into the phone workspace and export/share the phone workspace as ZIP;
- mobile UI chrome now exposes live API/PC/workspace state and dynamic badges for approvals, diagnostics, dirty Git state, running tasks and native waits;
- GitHub Actions Rust job installs the Linux GTK/WebKit/pkg-config dependencies required by the Dioxus mobile crate before workspace checks;
- isolated Android SDK slice lives under `tools/android/sdk/` (see `tools/android/README.md` and `DOWNLOAD_BUDGET.md`).

## Verified today

| Area | Current state |
|---|---|
| Build | Green |
| Tests | 135 mobile / 170 core / 3 pc-host |
| Mobile settings | Saved config is loaded into live turns and approval continuations |
| GitHub tools | Use token from saved settings first, environment variables second |
| Pairing | Online discovery promotes an active route; “Open PC workspace” persists it |
| Runtime | `MobileRuntimeConfig::default()` loads the saved active workspace when one exists |
| Diagnostics | Rust + TypeScript + Python paths exist; latest diagnostics are re-injected into the next turn |
| Files | Local and active PC workspace browsing use real file data; pending approval diffs are real |
| Tasks | Durable records, queue lifecycle, artifacts/logs, PC-host log capture, mobile task manager UI and PC running-task sync exist |
| Runtime HTTP API | PC-host exposes task list/log endpoints; mobile UI subscribes to live SSE task events for real-time updates |
| Termux workspace | Settings selector validates an absolute Termux path and activates a persisted Termux runtime connection |
| Workspace import/export | Files panel exposes project ZIP import/export over the core helpers; final Android host picker/share verification remains pending |
| MCP/skills | Config/manifest registries and mobile UI surfaces exist |
| Android bridge | Kotlin bridges + JNI + `android_host` callbacks + Dioxus `MainActivity`; local SDK in `tools/android/` |
| Mobile UI | Cockpit screens exist; latest chrome/nav pass compiles; final Android visual verification still pending |

## Implemented but still partial

- Android host: JNI bridge, callback JSON parsing, Dioxus `MainActivity` (`WryActivity` subclass), and Kotlin coordinator are in-repo; device/emulator verification remains.
- Final visual UI pass is still not verified on device/emulator because the local environment currently lacks Dioxus CLI (`dx`).
- Durable tasks have records/UI, artifacts/logs, PC-host process start/stop/list/log RPCs and live SSE subscription; mobile UI updates in real-time without manual polling.
- MCP: HTTP + stdio connect, proxy tools in the registry, and engine injection work; long-lived stdio session reuse and device-side verification remain.
- Skills context is injected into engine turns; enabled skill state persists in `skills-state.json`.
- Terminal UI state persists recent sessions/output as closed sessions after restart; live process resurrection is intentionally not claimed.

## Highest-value remaining work

1. Install NDK + `dx` (~1.0–1.2 GB download; see `tools/android/DOWNLOAD_BUDGET.md`) and run device/emulator verification.
2. PC-host autostart/service installer on real machines.
3. Signed APK, release notes and store-ready packaging.

See `docs/PROJECT_AUDIT.md` for the detailed audit and `docs/MASTER_PLAN.md` for the execution backlog.

The intended phone/PC product organization is documented in `docs/PHONE_PC_OPERATING_MODEL.md`.
