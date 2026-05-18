# DeepSeek-Mobile — Roadmap (2026-05-18)

## Phase 0: Foundation ✅
- [x] Workspace structure (core/mobile/pc-host crates)
- [x] CI: `cargo check --workspace` on every push
- [x] MSVC toolchain, zero-compile workspace

## Phase 1: Core tool parity ✅ (mostly)
- [x] File ops: read, write, edit, delete, copy, move, list, read_many_files
- [x] Shell tool
- [x] Git tool: status, diff, commit, push, pull, branch, log, add, checkout, clone
- [x] Patch tool: apply_patch with rollback
- [x] Web tools: web_fetch, web_search (DuckDuckGo, ApprovalRequirement::Suggest)
- [x] Snapshot tools: create, list, restore, prune
- [x] GitHub tools: repo, PR, issue, browse, push_file
- [x] Tool registry with capabilities and approval routing
- [ ] GitHub tool UI surface (PC-side or remote-safe)

## Phase 2: Snapshots & rollback ✅ (mostly)
- [x] WorkspaceSnapshotService core
- [x] Pre-tool snapshot auto-create before approved writes/shell/git
- [x] Snapshot events in mobile timeline
- [x] Mobile restore panel with confirmation dialog
- [x] Snapshot pruning policy
- [ ] PC-gateway snapshot path for remote workspaces
- [ ] Post-turn auto-snapshot after successful file changes

## Phase 3: PC gateway & execution ✅ (mostly)
- [x] PC-host HTTP server with auth, path traversal protection
- [x] Read/write/list/exec/git stream operations
- [x] Command streaming via SSE (`/v1/gateway/exec/stream`)
- [x] Policy presets: ReadOnly / Developer / Admin
- [x] Multi-endpoint routing, health scoring, failover
- [x] PC pairing flow: ZIP launcher, .env, PowerShell/sh launcher
- [x] Mobile PC discovery (mDNS) and pairing panel
- [x] Terminal sessions on PC-host (open/input/close)
- [x] Terminal panel UI + native bridge events
- [ ] Terminal session persistence across app restarts
- [ ] Dev-server preview lifecycle
- [ ] PC-host autostart/service installer

## Phase 4: LSP & diagnostics ✅ (mostly)
- [x] PC-host diagnostics for Rust (`cargo check --message-format=json`)
- [x] Post-edit diagnostics hook for write/edit/apply_patch
- [x] Diagnostics severity mapping and mobile display
- [ ] TypeScript diagnostics (`tsc --noEmit`)
- [ ] Python diagnostics (`pyright`/`ruff`)

## Phase 5: Mobile UI 🔄 (in progress)
- [x] Chat screen with streaming reasoning
- [x] Cockpit layout (drawer + section panels)
- [x] PC pairing panel
- [x] Git panel (status/diff/branch/commit)
- [x] Snapshots panel (restore dialog)
- [x] Diagnostics panel (severity badges)
- [x] Terminal panel (session tabs, output, input)
- [x] Native bridge: document picker, PC discovery, terminal commands
- [ ] File tree explorer
- [ ] Diff/patch viewer
- [ ] Approval card screen
- [ ] Onboarding screen (DeepSeek API key)
- [ ] Settings screens (GitHub, cloud disks, security)
- [ ] Bottom navigation tabs

## Phase 6: Production polish (not started)
- [ ] DeepSeek API key onboarding + secure storage
- [ ] GitHub OAuth flow
- [ ] Real button wiring for Create ZIP / Share ZIP / Check PC
- [ ] Auto-commit/push with real GitHub repo
- [ ] Background task manager
- [ ] MCP/plugin host
- [ ] Termux bridge
- [ ] LSP: TypeScript + Python diagnostics
