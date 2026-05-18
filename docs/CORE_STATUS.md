# DeepSeek-Mobile — Core + Mobile Status

## Project state: active development
- **Date**: 2026-05-18
- **Workspace**: Cargo workspace with core/mobile/pc-host crates
- **Build**: `cargo check --workspace` — 0 errors
- **Tests**: 87 mobile tests pass; 4 pre-existing core env-dependent failures
- **Platform**: Windows (MSVC toolchain), target Android via Dioxus

## Core crate (deepseek-mobile-core)
### Completed
- DeepSeek API client (non-streaming + streaming SSE with reasoning tokens)
- Config model with full field set (API key, model, execution mode, GitHub, etc.)
- Agent wrapper with model routing primitives
- Mobile engine (MobileEngine) — turn orchestration, session history, tool loop
- Turn context and turn status tracking
- Agent events (Started, TextDelta, ReasoningDelta, ToolCallStarted, ToolCallFinished, etc.)
- Approval-aware tool loop with pending approval continuation
- Approval session grants and runtime persistence
- Runtime thread store (turns, approvals, decisions)
- Session persistence (JSON save/load, full conversation history)
- Post-turn auto-snapshot (auto-creates workspace snapshot after each successful turn)
- Tool registry with capabilities, approval routing, default mobile tool set
- ToolExecutionCoordinator — routes tools to LocalAndroid/Termux/PcGateway/RemoteYlit
- File ops: read, write, edit, delete, copy, move, list, read_many_files
- Shell tool (contract, routes through executor)
- Git tool: status, diff, commit, push, pull, branch, log, add, checkout, clone
- Patch tool: apply_patch with rollback
- Web tools: web_fetch (URL), web_search (DuckDuckGo) with Network capability + Suggest approval
- Snapshot tools: create, list, restore, prune; pre-tool auto-snapshot
- GitHub tools: repo, PR, issue, browse, push_file; GitHub REST API client
- PcGatewayClient with multi-endpoint routing, health scoring, failover
- Workspace Diagnostics Service (Rust cargo check for local/Termux)
- Auto-commit/push helper
- Events module with full AgentEvent enum

### In progress / planned
- TypeScript diagnostics (tsc --noEmit)
- Python diagnostics (pyright/ruff)
- PC-gateway snapshot path for remote workspaces
- MCP/plugin host
- Durable task queue

## Mobile crate (deepseek-mobile)
### UI panels completed
- Chat screen: streaming reasoning, text display, send input
- Onboarding panel: full-screen first-launch API key setup with validation
- Bottom navigation bar: 5 tabs (Chat/Files/Terminal/Git/Settings) with approval badge
- Cockpit layout: drawer + section panels with routing
- Approval panel: real approval cards with approve/session/deny actions
- PC pairing panel: full flow (configure, create ZIP, share ZIP, check connection)
- File tree panel: expandable directories, clickable navigation, up button, path display, preview
- Git panel: status, diff, branch, commit draft
- Snapshots panel: list, restore confirmation dialog, file count warning
- Diagnostics panel: severity badges, diagnostic cards
- Terminal panel: session tabs, output view, input field, native bridge events
- Settings panel: DeepSeek API key, model, execution mode, thinking level, GitHub
- Diff viewer: color-coded added/removed/context lines

### Native bridge
- Document picker (Kotlin bridge)
- PC gateway discovery (Android NSD/mDNS)
- Terminal commands (OpenTerminal, TerminalInput, CloseTerminal)
- Share file (enqueue_share_file for Android share intent)

## PC-host crate (deepseek-pc-host)
- HTTP server with /health and /v1/gateway/request endpoints
- Workspace grant model and path traversal protection
- File read/write/list directory
- Command execution with security policy and timeout
- Policy presets: ReadOnly, Developer, Admin
- Git operations: status, diff, commit, push, pull, branch
- Streaming command execution via SSE (/v1/gateway/exec/stream)
- Terminal sessions (open/input/close) with streaming output
- Rust diagnostics (cargo check --message-format=json)
- Task detection (Cargo.toml, package.json, pyproject.toml, pytest.ini)
- Request logs: /v1/gateway/logs, LogRing, operation tracking, latency

### Planned for PC-host
- Dev-server preview lifecycle
- Autostart/service installer
- TypeScript diagnostics
- Python diagnostics
