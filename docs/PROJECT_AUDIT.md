# DeepSeek Mobile project audit

Audit date: 2026-05-16

Reference project compared: `Hmbown/DeepSeek-TUI`.

## Executive summary

DeepSeek Mobile is not a full one-to-one port of DeepSeek TUI yet. It is a mobile-first Rust core plus Dioxus mobile UI, PC gateway, Android document bridge and a reduced but real agent/tool runtime.

The project already improves several mobile-specific areas that the terminal project does not target directly:

- Android/Dioxus cockpit UI instead of ratatui terminal UI.
- Approval cards suitable for touch UI.
- Android document picker bridge contract and Kotlin bridge module.
- PC companion gateway model for phone-to-PC execution.
- Mobile-safe workspace boundary checks.
- Attachment ingestion path for text/source files selected through Android sandbox copies.

However, the original DeepSeek TUI still has major subsystems that are not fully present:

- full CLI/dispatcher/runtime server split;
- full tool suite;
- MCP;
- skills;
- hooks;
- durable background task manager;
- LSP diagnostics;
- OS sandboxing;
- cost/prefix-cache telemetry;
- sub-agents/RLM;
- full web/github/search tools;
- complete snapshots integrated into engine turn lifecycle.

## Architecture comparison

### Original DeepSeek TUI

Workspace crates include:

- `crates/agent`
- `crates/app-server`
- `crates/cli`
- `crates/config`
- `crates/core`
- `crates/execpolicy`
- `crates/hooks`
- `crates/mcp`
- `crates/protocol`
- `crates/secrets`
- `crates/state`
- `crates/tools`
- `crates/tui`
- `crates/tui-core`

Runtime surface:

- dispatcher CLI `deepseek`;
- companion TUI binary `deepseek-tui`;
- ratatui interface;
- async engine;
- OpenAI-compatible streaming client;
- typed tool registry;
- durable task queue;
- runtime HTTP/SSE API;
- LSP post-edit diagnostics;
- MCP and skills.

### DeepSeek Mobile

Workspace crates currently include:

- `crates/core`
- `crates/mobile`
- `crates/pc-host`

Additional Android module:

- `android/bridge`

Runtime surface:

- Dioxus mobile UI;
- mobile engine;
- DeepSeek streaming client;
- reduced tool registry;
- runtime store JSON files;
- PC gateway client and host;
- Android document picker bridge;
- mobile project file viewer;
- mobile approval cards;
- workspace snapshot service.

## Feature matrix

| Area | Original DeepSeek TUI | DeepSeek Mobile status | Notes |
|---|---|---|---|
| Streaming model output | Full streaming + reasoning blocks | Mostly done | Mobile engine now uses `run_stream`; UI merges deltas into one live assistant bubble. Reasoning-specific blocks are not fully separated from normal text yet. |
| Approval UI | Terminal approval dialog/cache | Done mobile-style | Touch approval cards exist with approve once/session/deny/abort. |
| Continue after approval | Done | Done | Runtime store pending approvals and UI continuation exist. |
| File tools | Full file/read/edit/apply patch stack | Partial | read/write/list/edit/file_ops exist. apply_patch is not yet implemented as a first-class tool. |
| Shell | Real sandboxed shell | Partial | PC-host can execute commands; mobile local shell is intentionally disabled. Termux bridge is not production yet. |
| Git | Rich git tools | Partial | PC-host supports status/diff and generic git command route; no Git UI. |
| Web/search/fetch | Rich web/search tools | Missing | Not yet ported to mobile core. |
| GitHub tools | Guarded gh-backed tools | Missing/partial | No GitHub UI/tool surface in mobile app. |
| MCP | Full MCP crate | Missing | Not ported. |
| Skills | Full skills system | Missing | Not ported. |
| Hooks | Pre/post lifecycle hooks | Missing | Not ported. |
| Background task manager | Durable queue | Missing | PC-host detects tasks but no durable task manager. |
| Runtime API | HTTP/SSE app server | Missing | PC-host has gateway HTTP API, not full agent runtime API. |
| LSP diagnostics | Fully wired post-edit hook | Missing/partial | PC protocol has diagnostics type; PC-host returns empty diagnostics. |
| Snapshots/rollback | Side-git snapshots + restore/revert_turn | Newly added core service | File-copy snapshot service now exists, but not yet wired into engine turns/UI. |
| OS sandbox | Seatbelt/Landlock/Windows | Missing | Workspace path policy exists, not OS sandbox. |
| Context compaction | 1M context, compaction, prefix-cache | Partial | Context budget/plans exist, not fully tied to engine/model turns. |
| Cost tracking | Full per-turn/cache telemetry | Missing | Token estimation only. |
| Model auto-routing | Auto model/thinking | Partial | Basic mobile router exists. Not equivalent to original auto-routing. |
| Sub-agents | Persistent sub-agent sessions | Missing | Not ported. |
| RLM | Persistent REPL sessions | Missing | Not ported. |
| Attachments | Not mobile-specific | Improved mobile path | Android bridge + text/source ingestion. PDF/DOCX/OCR still missing. |
| Android picker | Not applicable | Mostly done | Kotlin bridge module exists; final host integration still required. |
| PC gateway | Not original focus | Partial improvement | Phone-to-PC execution path exists and is useful, but needs UI/pairing hardening and diagnostics. |

## Current project strengths

1. Mobile UI foundation is real: Dioxus app, drawer, sections, timeline, approval panel.
2. Core approval flow is not a stub: pending approvals are persisted and can be continued.
3. Streaming pipeline now uses DeepSeek SSE through `DeepSeekAgent::run_stream`.
4. PC-host can perform real workspace file operations, command execution, git status/diff and task detection.
5. Android document picker has a real Kotlin bridge module for sandbox-copying `content://` URIs.
6. Attachments can inject local UTF-8 text/source content into prompt text.
7. Workspace boundaries reject path traversal.
8. Snapshot/rollback core service now exists.

## Major gaps to close next

### P0 — Build correctness and runtime wiring

- Run `cargo check --workspace` and fix compile errors.
- Add CI workflow for Rust checks and Android bridge lint/build if possible.
- Ensure `ToolExecutionCoordinator` is actually used inside `tool_loop`, not bypassed by direct `registry.execute`.
- Persist approval-session grants across turns; current `MobileEngine` methods clone the session and do not store new grants in `self`.

### P1 — Tool parity with DeepSeek TUI core agent behavior

- Add `apply_patch` tool.
- Add delete/move/copy file tools or extend `file_ops`.
- Add web/search/fetch tool surface where safe for mobile.
- Add richer git operations and Git UI.
- Add hooks equivalent for pre/post tool execution.

### P2 — Snapshots integration

- Create pre-turn snapshots before write/shell/git operations.
- Create post-turn snapshots after successful turns.
- Add restore UI in mobile cockpit.
- Add `restore_snapshot`/`list_snapshots` model-visible tools.

### P3 — LSP diagnostics

- Implement PC-host diagnostics using available tools:
  - `cargo check --message-format=json` for Rust;
  - `npm`/`tsc`/`eslint` when package files exist;
  - `pytest`/`ruff`/`pyright` when Python config exists.
- Surface diagnostics in mobile UI and inject them into next model turn.

### P4 — Background tasks

- Add durable task records and queue.
- Reuse PC-host task detection.
- Add mobile task manager UI.
- Persist task timeline/artifacts.

### P5 — MCP/plugins/skills

- Add minimal plugin/skill host data model.
- Add MCP client later, likely PC-side first.
- Add bundled starter skills only after core host is stable.

## Transfer conclusion

Nothing indicates that all original DeepSeek TUI functionality has been fully transferred. The mobile project has correctly transferred and adapted the central ideas — streaming, approval gates, tool registry, workspace execution boundaries, runtime persistence and PC companion execution — but the original project still contains many mature subsystems that are missing or only represented by placeholders.

The correct path is not to copy the original repo wholesale. The mobile version should keep its phone-first architecture and selectively port missing subsystems behind mobile/PC-safe abstractions.
