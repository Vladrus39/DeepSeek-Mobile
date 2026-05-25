# DeepSeek-Mobile project audit

**Audit refreshed:** 2026-05-25

**Reference project:** `Hmbown/DeepSeek-TUI`

## Executive summary

DeepSeek-Mobile has moved beyond a prototype. The project now has a real mobile-first runtime, functioning PC gateway, approvals, snapshots, diagnostics, persisted settings, Git/GitHub tooling, durable task records, task UI, MCP/skills registry surfaces, and a mostly complete cockpit UI.

The main remaining work is no longer “port the basics from the TUI.” It is now about **production closure**:

- verify and harden the final Android host adapter;
- finish Android project import/export UI over the new core ZIP helpers;
- extend the runtime/task model with SSE/live updates and stronger reconciliation;
- add release packaging and troubleshooting material.

## What is solidly implemented

### Runtime and safety

- DeepSeek streaming with reasoning deltas
- Session/runtime persistence
- Approval storage and continuation
- Workspace boundaries and tool capability gating
- Saved mobile settings applied to real turns and approval continuations
- GitHub token propagation from saved settings into the tool context
- Model routing and context fitting wired into turn orchestration

### Tools and editing

- File read/write/edit/delete/copy/move/list
- `apply_patch` operation batches and unified-diff input
- Shell, git, web, GitHub tools
- Snapshot create/list/restore
- Pre-tool and post-turn snapshot hooks
- Auto-commit/push lifecycle when enabled

### PC execution path

- Authenticated PC-host gateway
- Endpoint candidate planning, health scoring, and failover
- Pairing ZIP generation
- mDNS discovery
- Command streaming
- Git operations
- Terminal sessions
- Host logs and health
- PC snapshot RPC routing
- Background task start/stop/list RPC routing
- Rust / TypeScript / Python diagnostics

### Mobile surfaces

- Chat timeline and approval cards
- Onboarding/settings
- Files with real tree/preview, real pending diffs, and active-PC-aware browsing
- Snapshots, diagnostics, terminal, PC host, Git, tasks, MCP, and Skills panels
- Global mobile chrome with live API/PC status, active workspace summary and dynamic badges for approvals, diagnostics, dirty Git state, running tasks and native waits
- Native bridge contracts for document picker, discovery, terminal, sharing, and Termux `RUN_COMMAND` execution
- Termux result continuation from callback output back into the model turn
- Termux workspace selector that persists and activates a runtime Termux connection
- Core workspace ZIP import/export helpers with archive traversal protection

## Important improvements completed in the latest tranche

1. **Later-phase local commits were audited before continuing.**

   The local branch already contained durable tasks, task UI, PC snapshot routing, remote-aware Files, MCP/skills UI and terminal persistence work. The audit avoided duplicating those features.

2. **Termux workspace selection now activates a real backend.**

   The Settings panel now validates an absolute Termux path, persists it, and activates a `WorkspaceConnection` so future turns can route execution through Termux. Saved configs are revalidated on load instead of being trusted blindly.

3. **Core workspace import/export helpers were added and hardened.**

   ZIP import/export now exists in core. Import rejects parent traversal, absolute paths, Windows drive prefixes and backslash traversal. Export emits portable ZIP entry names and excludes `.deepseek-mobile` metadata.

4. **`apply_patch` now accepts unified diffs.**

   The tool still uses the safe operation model internally, but callers can now provide standard unified diff text through `unified_diff` or `patch`. PC-gateway routing normalizes the same input before remote execution.

5. **PC-aware Files routing was hardened.**

   The Files panel now passes the active `WorkspaceConnection.workspace_id` into PC-gateway file reads/lists instead of accidentally using the display root as the workspace id.

6. **Terminal UI-state persistence was made safer.**

   Saved terminal sessions load only once, save directories are created, restored sessions come back closed, and output truncation reports the real dropped-line count.

7. **Documentation was brought back in sync.**

   README, project status, roadmap, core status, progress log and master plan now reflect completed PC snapshots, tasks, artifacts/logs, runtime task HTTP API, MCP/skills, Termux continuation, Termux workspace activation, remote Files and unified-diff work.

## Partial implementations that still need completion

| Area | Current reality | What remains |
|---|---|---|
| Android host | Rust/Kotlin bridge contracts and integration notes exist | Final Dioxus host adapter and emulator/device verification |
| Visual UI verification | Cockpit screens and dynamic chrome exist; code-level checks pass | Real Android render/touch verification through Dioxus CLI and device/emulator |
| Termux | Native request queue, callback correlation, model continuation and Settings workspace activation exist | Final Android host verification |
| Android files | Chat attachment ingestion and core ZIP import/export helpers exist | Native project import/export UI flow |
| Terminal | PC-host sessions and persisted mobile UI history exist | Live terminal process resurrection is not claimed; service-level behavior can be improved later |
| Durable tasks | Core records, queue lifecycle, artifacts/logs, PC task RPCs and mobile UI exist | Live PC-running-task synchronization/reconciliation |
| MCP/skills | Registry/config/UI/context surfaces exist | Actual external MCP tool execution must be added carefully behind approval/workspace boundaries |
| Runtime API | PC-host HTTP task list/log endpoints exist | SSE/live event streaming still missing |
| Packaging | Development flow works | Android release notes, PC-host binary/service notes and troubleshooting docs |

## Comparison with DeepSeek-TUI

| Area | Mobile status |
|---|---|
| Streaming | Done |
| Approval workflow | Done |
| File editing | Done |
| Patch application | Done, including unified diff compatibility |
| Shell execution | PC-host done; Termux Rust/mobile continuation done; final Android host verification pending |
| Git tooling | Core, PC routing, mobile panel actions and auto-commit lifecycle done |
| Web/GitHub tools | Done in core |
| Diagnostics | Providers and model reinjection done |
| Snapshots | Local and PC-gateway paths done |
| Runtime API | Partial: PC-host task list/log HTTP endpoints done; SSE/live events pending |
| Durable tasks | Partial: records/queue/UI/RPC/artifacts/logs done; live sync pending |
| MCP/skills/plugins | Partial: registry/config/UI/context surfaces done; external tool execution pending |
| Android-native execution | Partial: contracts done, final host adapter pending |

## Recommended next execution order

1. Final Dioxus Android host adapter and emulator/device verification.
2. Android project import/export UI over the new core ZIP helpers.
3. Runtime SSE/live event streaming over existing runtime/task state.
4. Live PC-running-task reconciliation beyond task log capture.
5. Dev-server lifecycle, PC-host service/autostart and release/troubleshooting docs.

## Audit conclusion

The project is architecturally coherent. The main risk is now **capability drift**: implemented subsystems can become misleading if docs/UI claim more than the runtime really verifies. The right strategy is to keep closing vertical slices with tests, documentation and GitHub synchronization at each stable checkpoint.
