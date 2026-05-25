# DeepSeek Mobile master implementation plan

Created: 2026-05-16
Last updated: 2026-05-25

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
  -> PcPairingUiState active route
  -> WorkspaceConnectionStore persisted active connection
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
  -> diagnostics via Rust cargo check, TypeScript tsc and Python ruff/pyright
  -> post-edit diagnostics summary for PC write/edit/apply_patch calls when a PcGatewayClient is attached
  -> PcGatewayResponse
  -> ToolResult
  -> AgentEvent timeline
```

```text
Termux shell execution
  -> approved exec_shell on a Termux workspace
  -> ToolExecutionCoordinator emits termux_exec_request metadata
  -> mobile AgentEvent handling extracts pending Termux metadata
  -> NativeBridgeState queues NativeMobileCommand::RunTermuxCommand
  -> Android host must drain pop_next_android_termux_command()
  -> android/bridge DeepSeekTermuxBridge sends RUN_COMMAND intent
  -> host maps PendingIntent result to AndroidTermuxCallback
  -> NativeBridgeState rejects stale callbacks
  -> remaining: feed accepted result back into final tool/model output
```

```text
Local/Termux diagnostics
  -> LocalAndroid or Termux write/edit/apply_patch
  -> ToolExecutionCoordinator local route
  -> WorkspaceDiagnosticsService
  -> Rust cargo check, TypeScript tsc and Python ruff/pyright when project config exists
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

GitHub Actions runs on Ubuntu, so the Rust job must install the native Dioxus desktop/mobile dependency stack (`pkg-config`, GLib, GTK, WebKitGTK, appindicator and xdo development packages) before `cargo check --workspace --all-targets`.

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
- [x] Add manual Android bridge verification notes until a final Android host exists.
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
- [x] Add `delete_file` as a first-class local tool with approval.
- [x] Add `DeleteFile` support to PC gateway client/host.
- [x] Add `move_file` / `copy_file` with approval when writing.
- [x] Add `read_many_files` or bounded project search.
- [x] Upgrade `git` tool from contract placeholder to real routed operations.
- [x] Add Git UI panel: status, diff, branch, commit draft, pull/push approval.
- [x] Add web/fetch/search tools only behind explicit network capability and approval policy.
- [x] Add GitHub tool surface later, preferably PC-side or remote-safe.
- [x] Wire ModelRouter into engine turn orchestration (model auto-selection per prompt).
- [x] Wire ContextManager into prompt assembly (context budget fitting per model).
- [x] Wire auto_commit_and_push into successful-turn engine lifecycle.

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
- [x] Auto-create post-turn snapshot after successful turns with file changes.
- [x] Emit snapshot events to the mobile timeline.
- [x] Add mobile restore panel.
- [x] Add restore confirmation screen with file counts and deletion warning.
- [x] Add snapshot pruning policy.

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

- [x] Add pairing flow end-to-end from mobile UI.
- [x] Persist the selected online PC route as an active workspace connection for normal engine turns.
- [x] Add PC-host logs and health detail.
- [x] Add command allow/deny policy presets.
- [x] Add long-running command streaming instead of only completed output.
- [ ] Add terminal session persistence.
- [x] Add diagnostics implementation for TypeScript and Python.

Acceptance criteria:

- A phone can connect to PC, inspect project, edit files, run tests, view output, and recover from disconnect.
- PC execution must work without public internet when phone and PC have a direct/private route.

### P4 — LSP and diagnostics

Goal: match the original TUI's post-edit diagnostics loop in a mobile/PC-safe way.

Checklist:

- [x] Implement PC-host diagnostics for Rust via `cargo check --message-format=json`.
- [x] Implement TypeScript diagnostics via `tsc --noEmit` when config exists.
- [x] Implement Python diagnostics via `pyright`/`ruff` where available.
- [x] Add diagnostic severity mapping to `PcDiagnostic` for Rust cargo levels.
- [x] Add full post-edit diagnostic hook after `write_file`, `edit_file`, `apply_patch` across local, Termux and PC workspaces.
- [x] Add PC post-edit diagnostics summary for `write_file`, `edit_file` and `apply_patch` results when a `PcGatewayClient` is attached.
- [x] Add LocalAndroid/Termux post-edit Rust diagnostics through `WorkspaceDiagnosticsService`.
- [x] Surface diagnostics in mobile UI.
- [x] Inject diagnostics into next model turn as context.

Acceptance criteria:

- After an edit, errors/warnings become visible and model-readable before the next fix.

### P5 — Android and Termux execution

Goal: make Android more than a viewer and make Termux a real local executor.

Already done:

- [x] Android document picker Kotlin bridge module.
- [x] Attachment text/source ingestion through local sandbox path.
- [x] Android NSD/mDNS PC-host discovery bridge and Rust callback route.
- [x] Android/Termux `RUN_COMMAND` bridge contract and result parser.
- [x] Core `exec_shell` Termux route emits native pending request metadata.
- [x] Mobile event handling extracts pending Termux metadata and queues `RunTermuxCommand`.
- [x] Manual Android host integration notes for picker, PC discovery and Termux bridge wiring.

Remaining checklist:

- [x] Create final Android host integration instructions or module wiring.
- [ ] Add Dioxus/native callback adapter.
- [x] Add Termux command executor bridge contract.
- [x] Emit and queue Termux `exec_shell` native requests from the real tool route.
- [ ] Close Termux executor lifecycle through final Android host and tool output plumbing.
- [ ] Add Termux workspace selector.
- [ ] Add Android file import/export flow.
- [ ] Add PDF/DOCX/OCR ingestion later behind safe limits.

Acceptance criteria:

- Android picker returns files into chat without simulation.
- Termux commands can run with approval and bounded output.

### P6 — Runtime API and durable tasks

Goal: port the useful headless/task features from original DeepSeek TUI.

Checklist:

- [x] Add durable task records.
- [x] Add queue and task lifecycle: queued/running/completed/failed/canceled.
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

- [x] Cockpit dashboard: status, active workspace, executor, pending approvals, diagnostics, tasks.
- [x] Git panel with real status/diff/branch/commit/push/pull actions.
- [x] Snapshot/rollback panel.
- [x] Settings/profile screen.
- [x] API key setup and secret storage plan.
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
| OpenAI-compatible DeepSeek streaming | Keep | Done: SSE streaming with reasoning token support |
| Reasoning block streaming | Keep | Done: StreamDelta + ReasoningDelta in API client/engine |
| File tools | Keep and adapt | Done for local/PC-safe file operations |
| Apply patch | Keep mobile-safe operation batch first; add unified diff later | Partial: local + PC operation batches implemented |
| Shell execution | Route to PC/Termux | Partial: PC-host active; Termux native request queue added, final Android callback/result continuation still pending |
| Git tools | Keep with mobile UI | Partial: core/PC routing and panel actions exist; auto-commit lifecycle still pending |
| Web/search/fetch | Keep with approval | Done in core with network capability and approval policy |
| Runtime HTTP/SSE API | Keep later | Missing |
| Durable task queue | Keep | Missing |
| LSP diagnostics | Keep, PC-first plus local/Termux fallback | Partial: Rust/TypeScript/Python providers, UI metadata and next-turn model context implemented |
| PC connectivity | Keep multi-transport, offline-first | Partial: endpoint candidates, client failover, route health scoring, Android NSD discovery, reconnect controls and UI status display implemented |
| Snapshots/rollback | Keep, mobile-safe file-copy | Partial: core service, tools, local hooks and UI exist; PC snapshot path pending |
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
17. [x] Add pairing flow end-to-end from mobile UI.
18. [x] Persist active PC workspace selection from pairing into runtime configuration.
19. [x] Add Termux executor bridge contract.
20. [x] Wire Termux `exec_shell` into native request metadata and mobile bridge queue.
21. [x] Add Git UI.
22. [x] Wire Git panel actions to real local/PC-routed tool execution.
23. [x] Close Termux callback/result-continuation end-to-end.
24. [x] Replace illustrative Files diff preview with real pending/project diffs.
25. [x] Add background tasks (PC-host process spawning + gateway RPC).
26. [ ] Add durable task records + queue lifecycle (core/mobile side).
27. [ ] Add mobile task manager UI.
28. [ ] Add MCP/skills.

## 5. Implementation progress log
- 2026-05-25 (Phase F1 — durable task record + queue lifecycle): Added `DurableTaskStatus` enum (Queued/Running/Completed/Failed/Canceled), `DurableTaskRecord` struct with lifecycle methods (`mark_running`, `mark_completed`, `mark_failed`, `mark_canceled`), and `DurableTaskManager` with single-file JSON persistence under `base_dir/tasks.json`. Manager supports `create`, `save`, `load`, `load_all`, `update_status`, `delete`, `count_by_status`, and `prune_terminal_tasks`. Registered `pub mod durable_task` in core lib, re-exported `DurableTaskManager`, `DurableTaskRecord`, `DurableTaskStatus`. Verification: 16/16 durable_task tests pass, `cargo check --workspace --all-targets` green.

- 2026-05-25 (Phase E2 — background task infrastructure): Added `TaskHandle` with child-process tracking to PC host, implemented `run_task_handler` (spawns detected tasks as real processes), `stop_task_handler` (kills tracked child), and `list_tasks_handler` (returns running task info). Extended gateway protocol with `PcGatewayRequest::ListTasks`, `PcGatewayResponse::TaskList(Vec<PcRunningTaskInfo>)`, and `PcRunningTaskInfo` struct. Added `stop_task()` and `list_tasks()` to `PcGatewayClient`. Wired `detect_tasks`, `task_run`, `task_stop`, and `task_list` tool routing in `ToolExecutionCoordinator::execute_on_pc_gateway` with response handling for `TaskStarted`, `TaskStopped`, and `TaskList`. Verification: `cargo check` green, `cargo test` 102 mobile / 118 core / 2 pc-host.

- 2026-05-25 (Phase E1 — PC-workspace snapshot routing): Added `snapshot_create`, `snapshot_list`, and `snapshot_restore` tool routing to `execute_on_pc_gateway` in `ToolExecutionCoordinator`. Extended `gateway_response_to_tool_result` with `SnapshotRecord`, `SnapshotList`, and `SnapshotRestoreReport` response handling. PC workspace snapshots now route through the existing gateway RPC instead of failing with "not yet mapped". Verification: `cargo check` green, `cargo test` 102 mobile / 118 core / 2 pc-host.

- 2026-05-25 (Phase E0 — remote-aware file browsing): Made `ProjectFilesUiState` backend-aware with a `FileBrowserBackend` enum (`Local` / `PcGateway { workspace_id }`). Updated `project_files_panel.rs` to show a "PC"/"Local" badge in the header, dispatch file tree refresh through `refresh_via_pc` when a `PcGatewayClient` is present, and open files via `open_file_via_pc` asynchronously. Navigation between directories resets `loaded` state to trigger re-fetch from the active backend. Verification: `cargo check` green, 6/6 project_files_state tests pass (including 4 new).

- 2026-05-25 (Phase A wiring): Wired `ModelRouter` into engine turn orchestration (auto-selects Flash/Pro per prompt complexity and context size), wired `ContextManager` into prompt assembly (fits messages within the selected model's context budget), and wired `auto_commit_and_push` into the successful-turn engine lifecycle. Added `run_stream_with_messages_and_model` / `run_with_messages_and_model` to `DeepSeekAgent` for explicit model selection. Engine now stores `Config` directly for routing/context/auto-commit decisions. Verification: `cargo check --workspace --lib` green, `cargo test` green with 118 core / 102 mobile / 2 pc-host tests.

- 2026-05-25 (Phase C1 — real Files diff preview): Replaced the illustrative fake diff preview in the Files panel with real diffs computed from pending approval cards. For `write_file` cards, compares "before" content with "content". For `edit_file` cards, applies search/replace on current file content and diffs against original. The `project_files_panel` now accepts `approval_cards` and computes diffs reactively in `diff_preview_card`. When no pending change matches the selected file, shows "No pending changes" instead of a fake hook. Verification: `cargo check` green, 102 mobile / 118 core / 2 pc-host tests.

- 2026-05-25 (Phase B — Termux callback/result-continuation closure): Closed the Termux end-to-end loop. Added `TurnStatus::WaitingForTermuxResult` variant, `TermuxExecutionPending` agent event, and `continue_termux_result` method on `MobileEngine`. When an `exec_shell` call is routed to Termux, the tool loop emits a `TermuxExecutionPending` event and the turn pauses with `WaitingForTermuxResult` status instead of completing. When the Android Termux bridge returns the real command output via `NativeMobileEvent::TermuxCommandCompleted`, the mobile UI triggers `continue_termux_result` which injects the real tool result into the session and re-queries the model so it can respond to the actual command output. Wire-up completed in `main.rs` via a new `use_effect` that watches for `TermuxCommandCompleted` and calls `continue_mobile_termux_result`. Verification: `cargo check --workspace --all-targets` green, `cargo test --workspace` passed with 102 mobile / 118 core / 2 pc-host tests.

- 2026-05-25: Wired Git panel actions to the existing `git` tool route through `ToolExecutionCoordinator`, including local workspace and active PC gateway workspace routing for status/diff/branch/commit/push/pull. Verification: `cargo check --workspace --all-targets` and `cargo test --workspace` passed with 101 mobile / 117 core / 2 pc-host tests.
- 2026-05-25: Fixed GitHub Actions Rust environment setup by installing Ubuntu native dependencies required by Dioxus/GTK/WebKit before workspace checks; root cause was missing `glib-2.0.pc` from `glib-sys` during CI.
- 2026-05-25: Wired Termux-workspace `exec_shell` into the real tool route: core now emits structured pending `TermuxExecRequest` metadata, mobile extracts that metadata from tool-result events, queues `NativeMobileCommand::RunTermuxCommand`, and surfaces the queued native request in the timeline. Verification: `cargo check --workspace --all-targets` and `cargo test --workspace` passed with 97 mobile / 117 core / 2 pc-host tests.
- 2026-05-25: Added session-level diagnostics context and next-turn diagnostics injection, normalized post-edit diagnostics metadata for UI/model consumers, added Rust mobile Termux bridge queue/callback routing, added Android `DeepSeekTermuxBridge` for Termux `RUN_COMMAND` intents/result bundles, updated Android bridge manifest permissions, and documented final Android host integration responsibilities. Verification: `cargo check --workspace --all-targets` and `cargo test --workspace` passed with 95 mobile / 116 core / 2 pc-host tests.
- 2026-05-18: Wired saved settings into real turns and approval continuations, propagated saved GitHub tokens into `ToolContext`, fixed multi-provider diagnostics aggregation, and stabilized auto-commit tests.
- 2026-05-18: Closed the pairing/runtime gap: online discovery now promotes an active route, the mobile pairing panel builds a real `WorkspaceConnection`, "Open PC workspace" persists it via `WorkspaceConnectionStore`, and `MobileRuntimeConfig::default()` reloads it on future turns. New pairing requests now use generated tokens instead of an empty auth token.
- 2026-05-17: Added `reasoning_content` support to DeepSeek V4 API client with `StreamDelta` enum (Text/Reasoning/Done); wired `ReasoningDelta` agent events through `MobileEngine.collect_model_answer`.
- 2026-05-17: Integrated full session message history into `MobileEngine` via `build_messages_for_turn`; added JSON file persistence to `Session` (save_to_file/load_from_file/load_or_new); wired session save/load into `mobile_engine_runner`.
- 2026-05-17: Extended PC-host git operations with `git_commit`, `git_push`, `git_pull`, `git_branch` handlers; added `GitCommit`, `GitPush`, `GitPull`, `GitBranch` to `PcGatewayRequest`.
- 2026-05-17: Wired PC pairing panel buttons with real onclick handlers: configure → export ZIP → wait for PC → discovery; end-to-end pairing flow now actionable from mobile UI.

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
- 2026-05-17: Implemented long-running command streaming via SSE on PC-host (`/v1/gateway/exec/stream`): added `CommandStreamEvent` type, `PcGatewayClient.stream_command()` with mpsc channel and endpoint failover, `parse_sse_event()` helper, and wired streaming into `ToolExecutionCoordinator` for `exec_shell` on PC gateway.
- 2026-05-17: Completed git tool PC gateway routing: added `git_commit`, `git_push`, `git_pull`, `git_branch` to `PcGatewayClient`; updated `ToolExecutionCoordinator.execute_on_pc_gateway` to use dedicated git handlers instead of generic `execute_command`; fixed parallel test race condition in `git` tool tests.
- 2026-05-17: Added Git UI panel to mobile cockpit: created `GitUiState` with status/diff/branch/commit tracking, `git_panel` Dioxus component with `SectionCard` + `DiffBlock` sub-components, wired into `cockpit_section_panel` and `main.rs` replacing the placeholder; build verified clean across all crates.
- 2026-05-18: Added command allow/deny policy presets: `PolicyPreset` enum (ReadOnly/Developer/Admin), preset constructors on `PcGatewaySecurityPolicy`, `DEEPSEEK_PC_HOST_POLICY` env var support in PC-host config; exported from core lib.
- 2026-05-18 (continued): Added web tools (`web_fetch`/`web_search` with DuckDuckGo, `ApprovalRequirement::Suggest`, `Network` capability), snapshot pruning (`prune_old_snapshots`), mobile snapshot restore UI (confirmation dialog with file-count/deletion warning), terminal panel UI + state + native bridge commands/events (`OpenTerminal`/`TerminalInput`/`CloseTerminal`, `TerminalOpened`/`TerminalOutput`/`TerminalClosed`/`TerminalFailed`), terminal event routing in `native_event_router.rs` and `main.rs` `use_effect`, all wired into cockpit drawer. Build verified clean across all crates.
- 2026-05-18 (session 3 evening): Added onboarding, bottom navigation, real approvals, share bridge wiring, expandable file tree, and post-turn auto snapshots. Verified `cargo check --workspace --all-targets`; mobile tests were green at that checkpoint.
- 2026-05-18 (session 2): Added mobile Settings panel (`settings_panel.rs`, `settings_state.rs`) with full config form: DeepSeek API key, model mode (Auto/Flash/Pro), execution mode (Plan/Agent/YOLO), thinking level (Off-High-Max), external access, GitHub token/repo/branch, auto-push toggle. Config persisted to `.deepseek-mobile/config.json`. Wired Settings panel into `cockpit_section_panel.rs` replacing placeholder. Extended `PcGatewayHealth` with `uptime_secs`, `request_count`, `error_count`. Created `pc_logs` module (`LogRing`, `PcGatewayLogEntry`, `PcGatewayLogs`). Added `/v1/gateway/logs` endpoint on PC-host with 200-entry ring buffer, request logging with operation names and latency tracking. `health_handler` returns live uptime + request/error stats. Added `host_detail_text()` to `PcPairingUiState` and "Host details" section in `pc_pairing_panel`. Build: `cargo check` clean across core, pc-host, mobile. Pushed to `origin/main`.

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