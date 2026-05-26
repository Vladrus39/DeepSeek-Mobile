# DeepSeek-Mobile project audit

**Audit refreshed:** 2026-05-26

Reference project: `Hmbown/DeepSeek-TUI`.

## Executive summary

DeepSeek-Mobile is now past the previous Android startup blocker. The debug Dioxus APK builds, installs and launches on a physical Android phone. The UI renders and the crash buffer is clean after smoke launch. The latest hardware run reaches the one-screen setup with API/Agent ready and Termux path still pending; with completed setup/saved workspace state, the app opens into the main cockpit with `API OK`.

The project has a coherent phone-first architecture:

- Rust core agent/runtime;
- Dioxus mobile cockpit;
- native Android bridge module;
- Termux execution path as the intended phone-native full-agent backend;
- optional PC Host for large repos and desktop toolchains.

The main remaining work is production closure, not foundation work:

1. manual native Android end-to-end flow verification;
2. signed Android release packaging;
3. PC Host release/service packaging;
4. MCP stdio/external tool execution hardening.

## What is solidly implemented

### Runtime and safety

- DeepSeek streaming with reasoning deltas.
- Session/runtime persistence.
- Approval storage and continuation.
- Workspace boundaries and tool capability gating.
- Saved mobile settings applied to real turns and approval continuations.
- GitHub token propagation from saved settings into the tool context.
- Model routing and context fitting in turn orchestration.

### Tools and editing

- File read/write/edit/delete/copy/move/list.
- `apply_patch` operation batches and unified-diff input.
- Shell, git, web and GitHub tools.
- Snapshot create/list/restore.
- Pre-tool and post-turn snapshot hooks.
- Auto-commit/push lifecycle when enabled.
- Large tool-output spill to `.deepseek-mobile/tool-output/`.
- `workspace_overview` and `file_summary` orientation tools.

### PC execution path

- Authenticated PC Host gateway.
- Endpoint candidate planning, health scoring and failover.
- Pairing ZIP generation.
- mDNS discovery.
- Command streaming.
- Git operations.
- Terminal sessions.
- Host logs and health.
- PC snapshot RPC routing.
- Background task start/stop/list RPC routing.
- Mobile PC running-task reconciliation through the Tasks panel.
- Rust / TypeScript / Python diagnostics.

### Mobile surfaces

- Chat timeline and approval cards.
- Onboarding/settings.
- Files with real tree/preview, real pending diffs and active-PC-aware browsing.
- Snapshots, diagnostics, terminal, PC Host, Git, tasks, MCP and Skills panels.
- Global mobile chrome with live API/PC status, active workspace summary and dynamic badges.
- Native bridge contracts for document picker, discovery, terminal, sharing and Termux `RUN_COMMAND`.
- Termux result continuation from callback output back into the model turn.
- Termux workspace selector that persists and activates a runtime Termux connection.
- Core workspace ZIP import/export helpers with archive traversal protection.
- Files panel project import/export UI for the local phone workspace.
- Tasks panel PC running-task sync and stop controls.

### Android packaging/startup

- Dioxus Android host activity is present.
- Kotlin bridge module is packaged into Dioxus builds through `manganis`.
- JNI exports match the Kotlin bridge package.
- Native bridge loads `libmain.so` correctly for Dioxus.
- Custom manifest prevents the observed startup/config-change crash.
- Android adaptive launcher icon resources are present.
- Android data directory is initialized under app-private storage before Dioxus UI startup.
- Debug `.env` API-key prefill is available for hardware testing and disabled for release builds.
- MCP startup no longer blocks an active Tokio runtime during render.
- `reqwest` uses rustls for Android compatibility.

## Latest checkpoint fixes

| Issue | Fix |
|---|---|
| APK crashed looking for `libdeepseek_mobile.so` | Load Dioxus `libmain.so` first |
| JNI callbacks did not match Kotlin package | Exports renamed to `Java_com_deepseek_mobile_bridge_NativeBridge_*` |
| Activity restart caused native mutex crash | Custom manifest handles full config changes including `assetsPaths` |
| Android OpenSSL dependency risk | Workspace `reqwest` switched to `rustls-tls` |
| Bridge module missing from generated Dioxus APK | Added `manganis::ffi("../../android/bridge")` metadata |
| App-private data path not initialized for Android | JNI initializes `<filesDir>/deepseek-mobile/` before UI startup |
| Debug device testing required manual key paste every reinstall | Optional debug `.env` prefill; release builds ignore it |
| Main cockpit panicked after API bootstrap | Removed runtime `block_on` from synchronous MCP tool loading |
| Missing app icon/favicon | Added adaptive icon resources and SVG favicon |

## Partial implementations that still need completion

| Area | Current reality | What remains |
|---|---|---|
| Android host | APK builds and launch smoke passes on hardware | Full picker/share/Termux/PC-discovery manual checklist |
| Visual UI | Cockpit/onboarding render path verified | Full touch-flow walkthrough and polish |
| Termux | Native request queue, callback correlation and model continuation exist | Permission/setup test with real Termux and safe command |
| Android files | Chat attachment, import/export and share plumbing exist | Hardware picker/share verification |
| Terminal | PC-host sessions and persisted mobile UI history exist | Live process resurrection after restart is not claimed |
| Durable tasks | Records, queue lifecycle, artifacts/logs, RPCs, UI and SSE subscription exist | No major known gap |
| MCP/skills | Registry/config/UI/context and proxy surfaces exist | Long-lived stdio reuse and external execution behind approvals |
| Packaging | Debug Android build works | Signed APK/AAB and PC Host release packaging |

## Comparison with DeepSeek-TUI

| Area | Mobile status |
|---|---|
| Streaming | Done |
| Approval workflow | Done |
| File editing | Done |
| Patch application | Done, including unified diff compatibility |
| Shell execution | PC Host done; Termux bridge built; Termux hardware callback verification pending |
| Git tooling | Core, PC routing, mobile panel actions and auto-commit lifecycle done |
| Web/GitHub tools | Done in core |
| Diagnostics | Providers and model reinjection done |
| Snapshots | Local and PC-gateway paths done |
| Runtime API | PC Host task list/log HTTP endpoints plus SSE/live events done |
| Durable tasks | Records/queue/UI/RPC/artifacts/logs and live task-event subscription done |
| MCP/skills/plugins | Partial: registry/config/UI/context surfaces done; external tool execution pending |
| Android-native execution | Startup/build verified; deeper native flows pending |

## Recommended next execution order

1. Run the native Android manual verification checklist on the connected phone.
2. Fix any issues found in picker/import/export/share/Termux/PC-discovery flows.
3. Add release signing and produce signed APK/AAB.
4. Package PC Host binaries and optional service/autostart installer.
5. Finish MCP stdio session reuse and external MCP execution behind approvals.

## Audit conclusion

The architecture is still aligned with the initial goal: phone-first coding agent, Termux as the main phone executor, optional PC Host for scale. The project has not drifted into “PC required.” The remaining risk is claiming full production readiness before the deeper native flows are verified on hardware.
