# DeepSeek-Mobile — Project Status

## Current goal

Build a real mobile-first DeepSeek coding agent based on the original DeepSeek-TUI runtime architecture,
without reducing it to a simple chat client. Mobile + PC — the phone is the cockpit, PC is an optional
power executor through multiple connection modes.

The target product is a mobile AI coding cockpit:

```text
Android app = control cockpit
DeepSeek API = reasoning/model layer
Rust core = reusable agent runtime
PC-host / Termux / remote executors = execution layer
GitHub / cloud disks = external project memory and sources
```

## Source architecture preserved from DeepSeek-TUI

The original runtime shape must remain visible in the mobile version:

```text
UI -> engine -> session/turn -> model -> tool calls -> approval -> tool execution -> tool result -> durable timeline
```

DeepSeek-Mobile replaces the terminal UI and desktop-only executor with Android UX, mobile approval cards,
local/Termux/PC-host execution, and project-aware mobile screens.

## Implemented or started

### Workspace and crates

- Rust workspace with `core`, `mobile`, and `pc-host` crates.
- Shared workspace dependencies.
- Dioxus mobile shell.
- Initial PC-host binary crate.

### Core agent runtime

- DeepSeek API client and basic config model.
- Agent wrapper and model routing primitives.
- Mobile engine layer.
- Turn context and turn status tracking.
- Agent events and tool timeline primitives.
- Tool-call parsing.
- Approval-aware tool loop.
- Pending approval continuation contract.
- Approval decisions and session approval grants.
- Runtime store records for threads, turns, pending approvals and approval decisions.
- UI approval card contract.

### Tools and execution contracts

- Tool registry and tool capability model.
- File operation tools: read, write, list, edit/file operation surface.
- Shell and git tool contracts.
- **Git tools**: status, diff, commit, push, pull, branch, log, add, checkout, clone.
- **GitHub tools**: github_repo, github_pr, github_issue, github_browse, github_push_file.
- **GitHub API client**: auth, repo info, branches, file content, PR management, issue tracking.
- **Auto-commit/push**: `auto_commit.rs` helper for persisting changes after each agent turn.
- Executor abstraction.
- PC gateway executor planning layer.
- Workspace connection manager and persistent workspace connection store.

### PC gateway / PC-host

- PC companion gateway protocol.
- PC gateway client.
- Local/offline transport modes: LAN, direct Wi-Fi, loopback, tunnel/internet as optional modes.
- PC-host HTTP endpoints: `/health` and `/v1/gateway/request`.
- PC-host workspace grant model.
- PC-host file read/write/list directory.
- PC-host command execution through security policy.
- PC-host git status/diff (can be extended to commit/push/pull).
- Optional bearer-token authentication.

### PC pairing flow

- Core `PcGatewayPairingBundle` contract.
- Pairing JSON generation.
- `.env` generation for PC-host.
- Windows PowerShell launcher generation.
- Linux/macOS launcher generation.
- Pairing folder writer.
- Pairing ZIP writer.
- Mobile `PcPairingManager`.
- Mobile `PcPairingUiState`.
- Dioxus `PcPairingPanel` component.

### Mobile UI

- Basic chat screen.
- Initial cockpit layout.
- PC-host pairing/status card.
- First visual direction: ChatGPT + Cursor + Replit style mobile cockpit.

### GitHub integration (new — 2026-05-17)

- GitHub config fields in `Config`: `github_token`, `github_repo`, `github_branch`, `auto_commit_push`.
- `GitHubClient`: REST API v3 wrapper with token auth, repo info, branches, file CRUD, PR/issues.
- Five GitHub tool specs: `github_repo`, `github_pr`, `github_issue`, `github_browse`, `github_push_file`.
- Extended `GitTool` to support 10 git operations including commit, push, pull, branch, clone.
- `auto_commit.rs`: helper to auto-commit and push workspace changes after successful turns.

## Still missing / incomplete

### Critical build quality

- Confirm full workspace build with `cargo check --workspace`. (Blocked: Windows GNU toolchain needs MSVC)
- Add and keep GitHub Actions CI green.
- Run tests for `core`, `mobile`, and `pc-host`.
- Remove any compile regressions introduced by new contracts.

### Core runtime

- True streaming response handling from the DeepSeek API.
- Reasoning block rendering.
- Full message-history handling in mobile engine.
- Stronger durable persistence layer, likely SQLite or a file-backed store with migration support.
- Snapshots integration into engine turn lifecycle.
- Large output routing and context promotion.
- MCP/plugin host.
- Background task manager.

### PC-host / execution

- Stronger workspace path hardening.
- Command timeout enforcement.
- Safe UTF-8 output truncation.
- Diagnostics request implementation (partial: cargo check JSON).
- Task detection from `Cargo.toml`, `package.json`, `pyproject.toml`, etc. (partial).
- Terminal sessions with streaming output.
- Dev-server preview lifecycle.
- Autostart/service installer for Windows, Linux and macOS.
- Extended git operations through PC-host (commit/push/pull — currently only status/diff).

### Mobile UI

- Onboarding screen for DeepSeek API key.
- **GitHub settings screen** (token, repo, auto-push toggle).
- Settings screens for DeepSeek, GitHub, cloud disks and PC-host.
- Real file tree.
- Diff/patch viewer.
- Approval card screen.
- Terminal output screen.
- Git panel.
- Bottom tabs: Chat / Files / Terminal / Git / Settings.
- Real button wiring for Create ZIP, Share ZIP and Check PC connection.

### Integrations

- GitHub OAuth/token flow (REST API client done; OAuth flow pending).
- Real GitHub repository browsing, commit/push/PR workflows (API + tools done; UI pending).
- Cloud disk provider interfaces.
- Termux bridge.
- Remote Y-lit executor.
- LSP diagnostics.

## Current implementation estimate

```text
Core / agent runtime         ~65-70%
Approval / risk model        ~70-80%
Runtime store / history      ~60-70%
Tool loop                    ~65-75%
File tools                   ~65-75%
Git tools                    ~70-80%  (was ~35-45% — expanded with 10 real ops)
GitHub tools                 ~60-70%  (NEW: 5 tools + API client)
PC gateway protocol/client   ~60-65%
PC-host daemon               ~25-35%
Mobile UI                    ~15-25%
Production-ready app         ~25-35%
```

## Immediate priorities

1. Fix build: switch to MSVC toolchain or add `dlltool` to GNU.
2. Wire GitHub token into mobile settings UI.
3. Harden PC-host path and command execution.
4. Wire Android UI buttons to pairing ZIP export and PC health check.
5. Add real DeepSeek API key onboarding and secure storage plan.
6. Add mobile timeline rendering for engine/tool/approval events.
7. Add file tree and diff viewer.
8. Add terminal streaming from PC-host.
9. Add Git/GitHub workflow screens.
10. Test auto-commit/push with real GitHub repo.

## Non-negotiable product direction

DeepSeek-Mobile must remain a real AI coding agent, not a simple chat wrapper.

The phone is the cockpit. The model thinks. The Rust core manages turns, tools and approvals.
PC-host/Termux/remote runtimes execute heavy work. Every risky operation must be visible and
confirmable from Android.
