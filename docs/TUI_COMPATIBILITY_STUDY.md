# DeepSeek-TUI Compatibility Study

This document maps the original `DeepSeek-TUI-Ylit` runtime to the Android-native `DeepSeek-Mobile` target.

## Source areas reviewed

- `docs/ARCHITECTURE.md`
- `crates/tui/src/core/engine.rs`
- `crates/tui/src/core/events.rs`
- `crates/tui/src/core/turn.rs`
- `crates/tui/src/tools/mod.rs`
- `crates/tui/src/tools/spec.rs`
- `crates/tui/src/tools/registry.rs`
- `crates/tui/src/tools/file.rs`
- `crates/tui/src/tools/apply_patch.rs`
- `crates/tui/src/tools/shell.rs`
- `crates/tui/src/tui/approval.rs`
- `crates/tui/src/client.rs`
- `crates/tui/src/llm_client/mod.rs`
- `crates/tui/src/runtime_api.rs`
- `crates/tui/src/runtime_threads.rs`
- `crates/tui/src/task_manager.rs`
- `crates/tui/src/snapshot/mod.rs`
- `crates/tui/src/config.rs`
- `crates/tui/src/tools/large_output_router.rs`

## What the TUI is

The original TUI is a local coding-agent runtime, not just a chat screen.

Its main runtime shape is:

```text
UI -> engine -> session/turn -> LLM stream -> tool calls -> approval -> tool execution -> tool result -> model -> final answer -> durable state
```

## Capabilities that Mobile must preserve

1. Engine and turn loop.
2. Event-driven streaming UI.
3. Tool registry and tool schemas.
4. File tools: read, write, edit, list, apply patch.
5. Approval and risk model.
6. Shell/job execution through a controlled executor.
7. Workspace boundary and trusted external folders.
8. Durable threads, turns and item timeline.
9. Runtime event replay.
10. Snapshots and rollback.
11. Large-output routing and context compression.
12. Provider/model capability matrix.
13. Auto Flash/Pro routing.
14. Git and GitHub tools.
15. MCP, skills and plugin surface.
16. Background task manager.
17. LSP diagnostics after file changes.

## Android migration strategy

Do not copy the terminal UI directly. Preserve runtime contracts and replace the presentation/execution surfaces.

Preserve:

```text
LlmClient
ToolSpec
ToolContext
ToolRegistry
TurnContext
AgentEvent
ApprovalDecision
RuntimeThread
TaskRecord
WorkspaceSnapshot
Executor
```

Replace:

```text
ratatui UI -> Dioxus Android UI
local desktop shell -> LocalAndroid / Termux / RemoteYlit executor
desktop paths -> workspace + Android user grants
terminal approvals -> mobile approval cards
terminal logs -> mobile timeline and log viewer
```

## Target DeepSeek-Mobile layers

```text
UI layer:
- chat
- project tree
- diff viewer
- tool timeline
- approval cards
- git panel
- settings/API key

Core layer:
- MobileEngine
- TurnContext
- Session/Thread store
- AgentEvent stream
- ModelRouter
- ContextManager
- ToolRegistry
- ApprovalPolicy

Execution layer:
- LocalAndroid executor
- Termux executor
- RemoteYlit executor

Integration layer:
- GitHub provider
- cloud disk providers
- MCP/plugin host
- Y-lit runtime API
```

## Current DeepSeek-Mobile state after mobile runtime work

Implemented or started:

- separate `core` and `mobile` crates;
- real DeepSeek API client wrapper;
- Dioxus Android chat shell;
- mobile drawer and cockpit sections;
- mobile chat composer with document attachment state;
- Android document picker contract;
- native mobile bridge command/event contract;
- `UserChatInput` / `UserAttachmentRef` core contract;
- mobile attachment mapping into core input;
- `MobileEngine::run_turn()` path used by mobile requests;
- turn creation and `TurnContext` lifecycle;
- `AgentEvent` primitives;
- `AgentEvent` to mobile timeline adapter;
- mobile timeline model and timeline card renderer;
- runtime event replay into mobile timeline on app startup;
- durable `RuntimeThreadStore` for threads, turns, events, pending approvals and approval decisions;
- mobile runtime config for thread id, runtime store path and workspace path;
- workspace boundary model;
- workspace file service;
- executor abstraction;
- PC gateway types and client surface;
- tool registry and tool schemas;
- file tools: `read_file`, `write_file`, `list_dir`, `edit_file`, generic file ops;
- shell tool contract;
- git tool contract;
- tool-call parsing and execution loop in the engine path;
- approval/risk primitives and approval card data model;
- approval session policy primitives;
- context compression planning;
- Auto Flash/Pro model router.

Partially implemented but not yet complete:

- true streaming: `AgentEvent::TextDelta` exists, but mobile currently receives batch engine output after `run_turn()` completes;
- approval continuation: pending approvals can be stored, but mobile has no Approve/Reject UI action wired to `continue_after_approval`;
- tool timeline: tool events can be mapped, but live per-step streaming to UI is not yet wired;
- local Android execution: executor abstraction exists, but Android/Termux execution needs production wiring and permission handling;
- PC gateway execution: core types/client exist, but mobile pairing and request execution are not yet fully wired through the active engine;
- durable timeline: events are replayed, but full thread list, thread switching and item-level timeline are not yet exposed in mobile UI;
- workspace UI: workspace boundaries exist, but project tree, diff viewer and editor screens are not yet implemented.

Still missing compared with original TUI:

- real streaming LLM client integrated with mobile timeline;
- native Android file picker callback implementation;
- attachment content ingestion: PDF text, source text, images metadata and ZIP/project import;
- approval cards with Approve / Reject / Approve once / Approve session buttons;
- approval continuation from mobile UI;
- snapshots and rollback;
- large-output routing / workshop promotion;
- background task manager;
- long-running job monitor;
- LSP diagnostics after file changes;
- git panel UI and GitHub provider integration;
- MCP / plugin / skills host;
- thread list, archived threads and thread switching;
- runtime log viewer;
- API key/settings screen;
- full Android storage permission flow;
- production Termux executor bridge;
- production RemoteYlit executor bridge.

## Updated implementation order

1. Add mobile approval cards and wire `continue_after_approval`.
2. Replace batch mobile request handling with event streaming from `MobileEngine`.
3. Implement native Android document picker callback and remove simulated picked document placeholder.
4. Add attachment ingestion pipeline: source text, ZIP project import, PDF text extraction metadata.
5. Add project tree, file viewer and diff viewer screens.
6. Wire PC gateway pairing into active `MobileEngine` runtime.
7. Add Termux executor bridge with permission and safety boundaries.
8. Add snapshots and rollback.
9. Add large-output routing.
10. Add background task manager and long-running job monitor.
11. Add git/GitHub UI integration.
12. Add LSP diagnostics.
13. Add MCP/plugins/skills host.

## Gap summary

The mobile port is no longer only a chat prototype. It now has the central runtime spine: `MobileEngine`, `AgentEvent`, tool loop, runtime store, event replay, timeline UI and attachment-aware chat input.

The largest remaining gap versus the original TUI is interactivity during execution: streaming, approvals, live tool progress, and continuation after user decisions. The second largest gap is execution surface maturity: Android/Termux/PC gateway tools need production wiring, not only core contracts.