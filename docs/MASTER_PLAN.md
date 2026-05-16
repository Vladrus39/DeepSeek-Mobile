# DeepSeek Mobile master implementation plan

Created: 2026-05-16
Last updated: 2026-05-17

This is the working plan for completing DeepSeek Mobile without losing any original DeepSeek TUI capability that matters for a phone-first coding agent.

Reference repository: `Hmbown/DeepSeek-TUI`.

Current project audit: `docs/PROJECT_AUDIT.md`.

## 0. Non-negotiable rules

1. Do not replace the mobile architecture with a blind copy of DeepSeek TUI.
2. Preserve mobile-first execution boundaries: Android local workspace, Termux, PC gateway, remote Y-lit.
3. Every destructive operation must pass approval unless explicitly in a safe auto-approved mode.
4. Every newly added subsystem must be connected end-to-end: core contract, tool/engine wiring, runtime persistence, UI surface or documented host integration.
5. No silent placeholders. A partial implementation must expose its missing runtime dependency clearly.
6. Every feature must have at least one test or a documented manual verification path.
7. Update this master plan after every closed implementation item.
8. Keep the project active in two synchronized places: the PC working copy and the GitHub repository.

## 1. Current system map

```text
Android/Dioxus UI
  -> mobile_engine_runner
  -> MobileEngine
  -> DeepSeekAgent
  -> DeepSeekClient SSE stream
  -> tool_call parser
  -> tool_loop
  -> approval policy/session
  -> ToolExecutionCoordinator
  -> ToolRegistry OR PcGatewayClient
  -> RuntimeThreadStore events/pending approvals
  -> Mobile timeline / approval cards
```

Native and external execution map:

```text
Android document picker
  -> android/bridge Kotlin ACTION_OPEN_DOCUMENT
  -> app-private sandbox copy
  -> NativeBridgeState callback
  -> route_native_mobile_event
  -> ChatComposerState attachments
  -> attachment_ingestion
  -> UserChatInput prompt text
```

```text
Android PC discovery
  -> android/bridge Kotlin NSD/mDNS discovery
  -> _deepseek-pc-gateway._tcp. service records
  -> NativeBridgeState PC discovery callbacks
  -> route_native_mobile_event
  -> PcPairingUiState discovery report
  -> PcGatewayDiscoveryService endpoint validation and /health probing
  -> PcGatewayClient route scoring/failover
```

```text
PC execution
  -> MobileRuntimeConfig.workspace_connection
  -> MobileEngine.with_workspace_connection
  -> PcGatewayClient
  -> endpoint_plan: direct/local routes first, tunnel/internet fallback later
  -> runtime endpoint health scoring: success/failure/latency/last error
  -> PcGatewayDiscoveryService converts mDNS/manual/subnet records to endpoint candidates and probes /health
  -> mobile PC pairing panel shows discovery candidates, active route, endpoint health rows and reconnect controls
  -> tool_loop *_and_pc_gateway functions
  -> ToolExecutionCoordinator.with_pc_gateway
  -> pc-host HTTP /v1/gateway/request
  -> workspace path policy
  -> read/write/list/exec/git/task detection
  -> remote apply_patch via PC read/write/delete operations with rollback
  -> diagnostics via cargo check JSON for Rust workspaces
  -> post-edit diagnostics summary for PC write/edit/apply_patch calls when a PcGatewayClient is attached
  -> PcGatewayResponse
  -> ToolResult
  -> AgentEvent timeline
```

```text
Local/Termux diagnostics
  -> LocalAndroid or Termux write/edit/apply_patch
  -> ToolExecutionCoordinator local route
  -> WorkspaceDiagnosticsService
  -> cargo check --workspace --message-format=json when Cargo.toml exists
  -> best-effort diagnostics metadata without failing the original edit
```

### 1.1 Local + GitHub operating model

Canonical PC working copy:

```text
C:\Users\vladi\Desktop\DeepSeek-Mobile
```

Remote repository:

```text
https://github.com/Vladrus39/DeepSeek-Mobile
```

One-command publish flow from the PC working copy:

```powershell
.\deploy.ps1 -Message "short change summary"
```

Current Windows verification commands for this PC:

```powershell
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace
```

Operating rules:

1. Develop from the desktop working copy on the PC.
2. Keep GitHub synchronized after every closed implementation item or stable checkpoint.
3. The deploy script stages all changes, creates a commit when needed, rebases from `origin/<current-branch>`, and pushes the current branch back to GitHub.
4. If local and remote history diverge, resolve conflicts in the desktop working copy before continuing implementation.
5. The master plan, progress log, and code must be updated together so local work and repository state never silently drift apart.
6. On this Windows machine, use the installed MSVC toolchain for local verification until the default GNU toolchain is fully repaired; the current GNU path still fails before project code because its `dlltool.exe` setup is not healthy.

Snapshot/rollback map:

```text
snapshot_create/snapshot_list/snapshot_restore tools
  -> WorkspaceSnapshotService
  -> .deepseek-mobile/snapshots by default
  -> manifest.json + copied files
  -> restore with extra-file removal
  -> approval required for restore
```

## 2. Master backlog by priority

### P0 — Build correctness and wiring integrity

Goal: repository must build, tests must run, and all already-added pieces must be connected through the real execution path.

Checklist:

- [x] Add CI step for `cargo check --workspace`.
- [x] Add CI step for `cargo test --workspace`.
- [x] Fix every compile/test failure surfaced by CI.
- [x] Verify `ToolExecutionCoordinator` is used by `tool_loop` for all tool calls.
- [x] Fix approval-session grant persistence across turns.
- [x] Add a lightweight system-map test for default tool registry names.
- [ ] Add manual Android bridge verification notes until a final Android host exists.
- [x] Make CI failure visible on every push and pull request.

Acceptance criteria:

- `cargo check --workspace` passes.
- `cargo test --workspace` passes.
- Session approval grants are not lost between turns in one engine instance or stateless mobile runner callback.
- The default registry includes file, shell, git, snapshot and patch tools.

### P1 — Core tool parity

Goal: close the most important original DeepSeek TUI tool gaps while keeping mobile/PC-safe execution.

Checklist:

- [x] Add `apply_patch` as a first-class tool.
- [x] Map operation-based `apply_patch` to PC gateway execution.
- [ ] Add optional unified-diff parser compatibility for `apply_patch`.
- [ ] Add `delete_file` as a first-class local tool with approval.
- [x] Add `DeleteFile` support to PC gateway client/host.
- [ ] Add `move_file` / `copy_file` with approval when writing.
- [ ] Add `read_many_files` or bounded project search.
- [ ] Upgrade `git` tool from contract placeholder to real routed operations.
- [ ] Add Git UI panel: status, diff, branch, commit draft, pull/push approval.
- [ ] Add web/fetch/search tools only behind explicit network capability and approval policy.
- [ ] Add GitHub tool surface later, preferably PC-side or remote-safe.

Acceptance criteria:

- The model can patch a file, show diff, request approval, apply patch, run diagnostics and rollback.
- Git operations are not just text placeholders.
- Network tools are never silently enabled.

### P2 — Snapshots and rollback integration

Goal: make rollback part of the normal agent lifecycle, not only standalone tools.

Already done:

- [x] `WorkspaceSnapshotService` core service.
- [x] `snapshot_create`, `snapshot_list`, `snapshot_restore` tools.

Remaining checklist:

- [x] Auto-create pre-tool snapshot before approved local write/shell/git operations.
- [ ] Add PC-gateway snapshot path for remote workspaces.
- [ ] Auto-create post-turn snapshot after successful turns with file changes.
- [x] Emit snapshot events to the mobile timeline.
- [ ] Add mobile restore panel.
- [ ] Add restore confirmation screen with file counts and deletion warning.
- [ ] Add snapshot pruning policy.

Acceptance criteria:

- User can restore a workspace state from the app UI.
- Destructive restore always requires approval.
- Snapshots are not stored inside user `.git`.

### P3 — PC gateway production path

Goal: make PC execution reliable enough to act as the phone's real development machine.

Already done:

- [x] PC-host HTTP server.
- [x] Auth token support.
- [x] Path traversal protection.
- [x] Read/write/list/exec/git status/git diff.
- [x] Task detection for Cargo/npm/pytest.
- [x] Rust diagnostics via `cargo check --message-format=json`.
- [x] Post-edit diagnostics summary for PC `write_file`, `edit_file` and `apply_patch` calls when routed through an attached `PcGatewayClient`.
- [x] Wire `PcGatewayClient` into `MobileEngine` / runtime configuration so PC workspace execution is reachable from normal tool_loop turns.
- [x] Add multi-endpoint PC gateway routing model for direct Wi-Fi, same-LAN, tunnel and internet candidates.
- [x] Add client-side endpoint failover: local/direct candidates are tried before tunnel/internet candidates.
- [x] Map `apply_patch` to PC gateway execution using remote read/write/delete operations with rollback.
- [x] Add active endpoint cache and route health scoring.
- [x] Add mobile PC connection status display with active route and endpoint health.
- [x] Add PC gateway discovery core contract for mDNS/manual/subnet candidates and mobile discovery display.
- [x] Add Android NSD/mDNS adapter for PC-host discovery.
- [x] Add reconnect controls for PC gateway.

Remaining checklist:

- [ ] Add pairing flow end-to-end from mobile UI.
- [ ] Add PC-host logs and health detail.
- [ ] Add command allow/deny policy presets.
- [ ] Add long-running command streaming instead of only completed output.
- [ ] Add terminal session persistence.
- [ ] Add diagnostics implementation for TypeScript and Python.

Acceptance criteria:

- A phone can connect to PC, inspect project, edit files, run tests, view output, and recover from disconnect.
- PC execution must work without public internet when phone and PC have a direct/private route.

### P4 — LSP and diagnostics

Goal: match the original TUI's post-edit diagnostics loop in a mobile/PC-safe way.

Checklist:

- [x] Implement PC-host diagnostics for Rust via `cargo check --message-format=json`.
- [ ] Implement TypeScript diagnostics via `tsc --noEmit` when config exists.
- [ ] Implement Python diagnostics via `pyright`/`ruff`/`pytest` where available.
- [x] Add diagnostic severity mapping to `PcDiagnostic` for Rust cargo levels.
- [x] Add full post-edit diagnostic hook after `write_file`, `edit_file`, `apply_patch` across local, Termux and PC workspaces.
- [x] Add PC post-edit diagnostics summary for `write_file`, `edit_file` and `apply_patch` results when a `PcGatewayClient` is attached.
- [x] Add LocalAndroid/Termux post-edit Rust diagnostics through `WorkspaceDiagnosticsService`.
- [x] Surface diagnostics in mobile UI.
- [ ] Inject diagnostics into next model turn as context.

Acceptance criteria:

- After an edit, errors/warnings become visible and model-readable before the next fix.

### P5 — Android and Termux execution

Goal: make Android more than a viewer and make Termux a real local executor.

Already done:

- [x] Android document picker Kotlin bridge module.
- [x] Attachment text/source ingestion through local sandbox path.
- [x] Android NSD/mDNS PC-host discovery bridge and Rust callback route.

Remaining checklist:

- [ ] Create final Android host integration instructions or module wiring.
- [ ] Add Dioxus/native callback adapter.
- [ ] Add Termux command executor bridge.
- [ ] Add Termux workspace selector.
- [ ] Add Android file import/export flow.
- [ ] Add PDF/DOCX/OCR ingestion later behind safe limits.

Acceptance criteria:

- Android picker returns files into chat without simulation.
- Termux commands can run with approval and bounded output.

### P6 — Runtime API and durable tasks

Goal: port the useful headless/task features from original DeepSeek TUI.

Checklist:

- [ ] Add durable task records.
- [ ] Add queue and task lifecycle: queued/running/completed/failed/canceled.
- [ ] Add mobile task manager UI.
- [ ] Reuse PC task detection.
- [ ] Add artifacts and logs per task.
- [ ] Add HTTP/SSE runtime API only after core task model is stable.

Acceptance criteria:

- Long jobs survive UI navigation and can be inspected later.

### P7 — MCP, plugins, skills

Goal: add extensibility only after the core mobile agent is stable.

Checklist:

- [ ] Add local skills manifest format.
- [ ] Add bundled mobile-safe starter skills.
- [ ] Add plugin host model.
- [ ] Add MCP client through PC gateway first.
- [ ] Add MCP UI for server status and tool list.

Acceptance criteria:

- Skills/plugins cannot bypass approval policy or workspace boundaries.

### P8 — UX completion and release packaging

Goal: turn the prototype into a usable product.

Checklist:

- [ ] Cockpit dashboard: status, active workspace, executor, pending approvals, diagnostics, tasks.
- [ ] Git panel.
- [ ] Snapshot/rollback panel.
- [ ] Settings/profile screen.
- [ ] API key setup and secret storage plan.
- [ ] Android build/release notes.
- [ ] PC-host binary/release notes.
- [ ] Troubleshooting docs.

Acceptance criteria:

- New user can install, pair PC or choose local workspace, chat, edit, approve, run tests, rollback.

## 3. Original DeepSeek TUI feature transfer tracker

| Original feature | Mobile decision | Status |
|---|---|---|
| Ratatui terminal UI | Replace with Dioxus mobile cockpit | In progress |
| CLI dispatcher | Not priority for phone app | Not ported |
| OpenAI-compatible DeepSeek streaming | Keep | Mostly done |
| Reasoning block streaming | Keep later | Partial |
| File tools | Keep and adapt | Partial |
| Apply patch | Keep mobile-safe operation batch first; add unified diff later | Partial: local + PC operation batches implemented |
| Shell execution | Route to PC/Termux | Partial |
| Git tools | Keep with mobile UI | Partial |
| Web/search/fetch | Keep with approval | Missing |
| Runtime HTTP/SSE API | Keep later | Missing |
| Durable task queue | Keep | Missing |
| LSP diagnostics | Keep, PC-first plus local/Termux fallback | Partial: Rust cargo diagnostics and post-edit hooks implemented; TS/Python/UI/model-context still pending |
| PC connectivity | Keep multi-transport, offline-first | Partial: endpoint candidates, client failover, route health scoring, Android NSD discovery, reconnect controls and UI status display implemented |
| Snapshots/rollback | Keep, mobile-safe file-copy | Partial: core service, tools, local pre-tool hook |
| OS sandbox | Replace/augment with executor policies | Missing |
| MCP | Keep, PC-first | Missing |
| Skills | Keep after core | Missing |
| Hooks | Keep after tool parity | Missing |
| Sub-agents | Later | Missing |
| RLM | Later | Missing |
| Cost/prefix-cache telemetry | Later | Missing |
| Notifications | Later | Missing |
| Themes/localization | Later | Partial/unknown |

## 4. Immediate execution order

The next implementation sequence is fixed:

1. [x] Strengthen CI from `cargo check` to `cargo check + cargo test`.
2. [x] Fix compile/test failures surfaced by CI.
3. [x] Fix approval-session grant persistence.
4. [x] Add `apply_patch` tool.
5. [x] Auto-create pre-tool snapshots before destructive approved local tools.
6. [x] Add PC diagnostics for Rust projects.
7. [x] Wire `PcGatewayClient` into normal `MobileEngine` turns.
8. [x] Add multi-endpoint PC gateway route candidates and client failover.
9. [x] Map `apply_patch` to PC gateway execution.
10. [x] Add full post-edit diagnostic hook across local, Termux and PC workspaces.
11. [x] Add active endpoint cache and route health scoring.
12. [x] Add mobile PC connection status display with active route and endpoint health.
13. [x] Add PC gateway discovery core contract and mobile discovery display.
14. [x] Add Android NSD/mDNS adapter for PC-host discovery.
15. [x] Add reconnect controls for PC gateway.
16. [x] Add snapshot/diagnostics UI panels.
17. [ ] Add pairing flow end-to-end from mobile UI.
18. [ ] Add Termux executor bridge.
19. [ ] Add Git UI.
20. [ ] Add background tasks.
21. [ ] Add MCP/skills.

## 5. Implementation progress log

- 2026-05-16: Added master audit in `docs/PROJECT_AUDIT.md`.
- 2026-05-16: Added CI `cargo check --workspace` and `cargo test --workspace` jobs plus Android bridge static checks.
- 2026-05-16: Fixed approval-session persistence for mutable `MobileEngine` and stateless mobile runner callbacks via `ApprovalSessionRuntimeStore`.
- 2026-05-16: Added operation-based atomic `apply_patch` tool and registered it in the default mobile tool registry.
- 2026-05-16: Added local pre-tool snapshots inside `tool_loop::execute_approved_call()` for destructive local/Termux tools; PC-gateway snapshot path remains separate.
- 2026-05-16: Implemented PC-host Rust diagnostics using `cargo check --workspace --message-format=json`, mapped cargo levels to `PcDiagnosticSeverity`, and added path filtering.
- 2026-05-16: Added PC post-edit diagnostics summary/metadata after `write_file` and `edit_file` calls inside `ToolExecutionCoordinator` when a `PcGatewayClient` is attached.
- 2026-05-16: Wired `PcGatewayClient` through tool_loop, `MobileEngine`, `MobileRuntimeConfig`, and mobile runner so normal turns can execute PC workspace tools when a `WorkspaceConnection` is supplied. UI pairing remains separate.
- 2026-05-16: Added PC gateway endpoint candidates for direct Wi-Fi, same-LAN, tunnel and internet routes; `PcGatewayClient` now attempts endpoints by priority so local/offline routes are preferred before tunnel/internet fallback.
- 2026-05-16: Added `DeleteFile` support to PC gateway client/host and mapped operation-based `apply_patch` to PC gateway execution with remote backup/rollback and post-edit diagnostics.
- 2026-05-16: Added `WorkspaceDiagnosticsService` and wired best-effort LocalAndroid/Termux post-edit Rust diagnostics after `write_file`, `edit_file`, `apply_patch`, and modifying `file_ops` calls.
- 2026-05-16: Added runtime PC gateway endpoint health scoring in `PcGatewayClient`, including success/failure counters, last latency, last error, active endpoint selection, and health-aware endpoint ordering.
- 2026-05-16: Extended PC pairing UI state and panel to show active PC route, endpoint health rows, latency, route score and last endpoint error.
- 2026-05-16: Added `PcGatewayDiscoveryService` for mDNS/manual/subnet discovery records, `/health` probing, discovery reports, and mobile panel display of discovery candidates.
- 2026-05-16: Added Android NSD/mDNS discovery bridge for DeepSeek PC Host, required Android network/multicast permissions, Rust native discovery payloads, and route_native_mobile_event integration into PcPairingUiState.
- 2026-05-16: Added PC gateway reconnect controls in PcPairingUiState and PcHost panel: scan again, retry active route, use best discovered route, and forget bad routes.
- 2026-05-17: Established the synchronized PC + GitHub operating model, created the desktop working copy at `C:\Users\vladi\Desktop\DeepSeek-Mobile`, and added the one-command `deploy.ps1` publish script.
- 2026-05-17: Stabilized P0 build integrity: removed obsolete direct `dioxus-mobile` usage in favor of stable `dioxus::launch`, consolidated duplicate Rust workflows into one CI path, added `Cargo.lock`, fixed snapshot/runtime/mobile API drift, made workspace path tests cross-platform, and verified `cargo check --workspace --all-targets` plus `cargo test --workspace` locally through the installed MSVC toolchain.
- 2026-05-17: Added first mobile snapshot and diagnostics surfaces: tool-result events now retain structured metadata, automatic pre-tool snapshots and post-edit diagnostics are echoed into the timeline, restored runtime events rebuild snapshot/diagnostics state on launch, and the drawer now exposes dedicated `Snapshots` and `Diagnostics` panels.

## 6. Definition of done for the project

DeepSeek Mobile reaches v1 when all of the following are true:

- Mobile app can stream responses live.
- User can attach files from Android picker and text/source content reaches the model.
- User can browse project tree and inspect files.
- Agent can propose changes and show diffs.
- User can approve/reject tools from touch UI.
- Agent can edit files and run tests through PC or Termux.
- Diagnostics appear after edits.
- User can rollback workspace changes.
- Git status/diff/commit workflow exists.
- Long-running tasks are durable.
- Network/MCP/skills are controlled by explicit policy.
- CI passes for core/mobile/pc-host.
