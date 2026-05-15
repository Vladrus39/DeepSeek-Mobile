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

## Implementation order

1. Stabilize workspace and CI.
2. Align mobile event model with TUI events.
3. Add mobile `ToolSpec`, `ToolContext`, `ToolResult`, and capabilities.
4. Implement real file tools.
5. Add approval policy.
6. Add `TurnContext` and snapshots.
7. Add streaming client and reasoning deltas.
8. Connect Auto Flash/Pro router to the agent.
9. Add durable thread/turn/item store.
10. Add Termux executor.
11. Add RemoteYlit executor.
12. Add git/GitHub tools.
13. Add task manager.
14. Add MCP/plugin support.
15. Add diagnostics through Termux or Remote executor.

## Current DeepSeek-Mobile state

Started:

- separate core and mobile crates;
- basic API client;
- basic mobile chat UI;
- session model;
- agent event primitives;
- workspace boundary;
- workspace file service;
- executor abstraction;
- context compression planning;
- Auto Flash/Pro router.

Missing compared with TUI:

- full engine loop;
- true streaming;
- tool-call parsing and execution loop;
- full tool abstraction;
- approval policy;
- turn context;
- snapshots/rollback;
- durable runtime store;
- Termux/remote execution;
- git/GitHub tools;
- MCP/plugins;
- diagnostics;
- background tasks;
- large-output workshop promotion;
- native screens for these features.
