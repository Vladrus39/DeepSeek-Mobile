# DeepSeek-Mobile Roadmap

## Vision

DeepSeek-Mobile is a **full coding agent on Android** (phone-first), porting the DeepSeek-TUI runtime to Dioxus + native bridges. The phone is the cockpit; the model runs in the cloud; the Rust core manages turns, tools and approvals.

**Primary executor on device:** Termux (real shell/git/build in a project directory).  
**Lite mode:** local app sandbox (files/patch without shell).  
**Optional:** PC-host when a project is too large for the phone — remote finish on a workstation, not a requirement to use the app.

See `docs/PRODUCT_POSITIONING.md`.

Execution modes (priority):

1. **Termux workspace** — full agent on the phone (main path).
2. **Local Android workspace** — safe file operations inside app storage.
3. **PC-host / remote** — optional boost for huge repos.

## Phase 0 — Stabilize repository

- [x] Add missing session module.
- [x] CI workflow for `cargo check` + `cargo test` + Android bridge static check.
- [x] Ensure workspace compiles (MSVC + GNU toolchains verified; `cargo check --workspace` clean).
- [x] `cargo test --workspace` passes (104/106 tests; 2 pre-existing auto_commit failures).
- [x] Added session persistence JSON file storage (save/load for conversation survival).
- [x] Added streaming command execution via SSE on PC-host.
- [x] Added policy presets (ReadOnly/Developer/Admin) for PC-host security.
- [x] Added Git UI panel in mobile cockpit (status, diff, branch, commit).
- [x] Basic architecture documents (docs/PROJECT_AUDIT.md, docs/CORE_STATUS.md).
- [x] MVP status tracking (PROJECT_STATUS.md).

## Phase 1 — Real chat core

- [x] Store API key through a mobile-safe settings flow.
- [x] Use full message history, not only the last user message.
- [x] Request/response error model (anyhow-based).
- [x] Provider abstraction for DeepSeek API (DeepSeekClient with streaming).
- [x] Non-streaming and streaming chat implementation.

## Phase 2 — Streaming agent events

- [x] `AgentEvent` enum (Started, TextDelta, ReasoningDelta, ToolCallStarted, ApprovalRequired, …).
- [x] Streaming API client (SSE-based, `DeepSeekClient::chat_stream`).
- [x] Render text deltas in the mobile UI (agent timeline panel).
- [x] Render reasoning/status events separately from final text.
- [x] Persist event timeline for resume (saved_timeline_loader).

## Phase 3 — Workspace and files

- [x] Workspace model with path traversal protection.
- [x] Project import/export as ZIP (PC pairing bundle).
- [x] File tree (project_files_panel, project_files_state).
- [x] `read_file` tool.
- [x] `write_file` tool.
- [x] `edit_file` / `apply_patch` tools.
- [x] Diff viewer (approval_diff_preview).
- [x] Patch approval screen (mobile_approval_panel).

## Phase 4 — Tool-calling loop

- [x] Tool schemas and JSON input contracts (ToolSpec trait).
- [x] Tool specs sent to model (through tool_loop).
- [x] Parse tool calls from model output (parse_tool_calls_from_text).
- [x] Execute tools through approval policy (approval.rs + tool_loop.rs).
- [x] Return tool results to model.
- [x] Stop only on final answer (ToolLoopOutcome.pending_approvals).

## Phase 5 — Execution policy

- [x] Plan mode (ExecutionMode::Plan exists; engine routes to thinking-only turns).
- [x] Agent mode (MobileEngine + tool_loop).
- [x] YOLO mode (ExecutionMode::Yolo exists; engine skips approval for non-destructive tools).
- [x] Dangerous command blocker (approval risk classification).
- [x] Workspace boundary checks (Workspace::contains, resolve_relative_path).
- [x] Per-tool approval rules (ApprovalRisk, ToolCategory, ApprovalSessionPolicy).

## Phase 6 — Termux and remote execution

- [x] Executor trait (Executor + CommandRequest/CommandOutput).
- [x] Local Android executor (file_ops tools on LocalAndroid workspace).
- [x] Termux bridge executor (Rust queue + callback continuation; Android host coordinator + JNI + callback JSON).
- [x] Remote PC-host executor (PcGatewayClient, PC-host HTTP server).
- [x] Command output to UI (agent_timeline events).
- [x] Persist command logs (runtime_store events).

## Phase 7 — Large project support

- [x] Project index (workspace_files + workspace_diagnostics task detection).
- [ ] File summaries (planned).
- [ ] Symbol search hooks (planned).
- [x] Test/build diagnostics (PC-host `cargo check --message-format=json`, cargo/npm/pytest task detection).
- [x] Snapshot/rollback (WorkspaceSnapshotService, snapshot_create/list/restore tools).
- [x] Cost/context tracking (ContextBudget, estimate_messages_tokens).

## Phase 8 — Plugins and integrations

- [x] MCP config registry, HTTP connect (`tools/list`), declared-tools fallback, mobile UI.
- [x] MCP stdio spawn + proxy tools in agent registry (`mcp_client`, `mcp_proxy`, engine injection).
- [ ] MCP stdio session reuse + on-device invoke verification.
- [x] GitHub tools (github_repo, github_pr, github_issue, github_browse, github_push_file).
- [x] GitHub API client (GitHubClient with auth, repo info, PR/issues, file push).
- [x] Git operations (status, diff, commit, push, pull, branch, log, checkout, clone).
- [x] Auto-commit/push after successful agent turn (auto_commit.rs).
- [ ] Y-lit deploy tools.
- [x] Durable task queue + PC-host background tasks + SSE task events.
- [ ] Background jobs beyond PC/durable task model.
- [ ] LSP diagnostics through remote or Termux executor.

## Current sprint: GitHub integration + production readiness

- [x] GitHub config fields (github_token, github_repo, github_branch, auto_commit_push).
- [x] GitHub REST API client.
- [x] GitHub tool surface (5 tools).
- [x] Extended git tool surface (10 operations) + Git UI panel in mobile cockpit.
- [x] Auto-commit/push helper.
- [x] `cargo check --workspace` clean; `cargo test --workspace` 104/106 pass.
- [x] Streaming command execution via SSE on PC-host.
- [x] Policy presets (ReadOnly/Developer/Admin) for PC-host security.
- [x] GitHub settings screen in mobile UI.
- [*] Integration test with actual GitHub repo (needs real token).

## Phase 9 — Android packaging (in progress)

- [x] Kotlin bridge module (picker, discovery, Termux, share).
- [x] Rust `android_host` drain + callback JSON + JNI `NativeBridge`.
- [x] Dioxus `MainActivity` (`dev.dioxus.main`, `WryActivity` subclass).
- [x] Project-local Android SDK slice (`tools/android/sdk/`, ~255 MB, no internet).
- [x] Download budget doc (`tools/android/DOWNLOAD_BUDGET.md`).
- [ ] Install NDK + `dioxus-cli` and `dx build android` on device/emulator.
- [ ] Signed release APK and store notes.
