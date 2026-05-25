# DeepSeek-Mobile — roadmap

**Updated:** 2026-05-25

## Phase 0 — Foundation ✅

- [x] Cargo workspace with `core`, `mobile`, and `pc-host`
- [x] CI/local build path for `cargo check` and `cargo test`
- [x] Runtime persistence and approval continuation
- [x] Persisted settings used by real turns

## Phase 1 — Core tool parity ✅

- [x] File ops, shell, git, web, GitHub, snapshots, `apply_patch`
- [x] `apply_patch` operation batches
- [x] `apply_patch` unified-diff compatibility
- [x] Capability and approval routing
- [x] Post-edit diagnostics hooks
- [x] Integrate `ModelRouter` into real turn selection
- [x] Integrate `ContextManager` into real prompt lifecycle
- [x] Invoke `auto_commit_and_push` from the engine when enabled

## Phase 2 — Snapshots & rollback ✅

- [x] Snapshot service
- [x] Pre-tool snapshots
- [x] Post-turn snapshots
- [x] Restore UI and pruning
- [x] PC-gateway snapshot path for remote workspaces

## Phase 3 — PC gateway & execution 🔄

- [x] HTTP host, auth, security policy, path protection
- [x] File/git/command operations
- [x] Streaming commands
- [x] Pairing ZIP, mDNS discovery, endpoint health/failover
- [x] Pairing persists the active PC workspace into runtime config
- [x] Rust/TypeScript/Python diagnostics in PC-host
- [x] Terminal sessions on PC-host
- [x] Terminal UI-state persistence across app restarts
- [x] PC-host background task start/stop/list RPCs
- [ ] Dev-server lifecycle
- [ ] PC-host autostart/service installer

## Phase 4 — Mobile UI ✅ / packaging pending

- [x] Chat, approvals, snapshots, diagnostics, settings, onboarding
- [x] PC host pairing surface
- [x] Files tree + preview
- [x] Replace illustrative Files diff preview with real pending/project diffs
- [x] Make file browsing remote-aware when a PC workspace is active
- [x] Terminal panel
- [x] Git panel with real operations
- [x] Durable task manager panel
- [x] MCP and Skills panels

## Phase 5 — Android & local execution 🔄

- [x] Native bridge contracts for picker, discovery, terminal, share
- [x] Termux `RUN_COMMAND` bridge contract
- [x] Termux `exec_shell` native request metadata and mobile queue extraction
- [x] Rust/mobile Termux result continuation back into the model
- [ ] Final Dioxus Android host adapter
- [ ] Device/emulator verification of host bridge callbacks
- [ ] Termux workspace selector
- [ ] Android import/export completion

## Phase 6 — Product completion 🔄

- [x] Diagnostics injected into the next model turn
- [x] Durable background task records and task manager UI
- [x] MCP/skills config and UI layer
- [x] PC pairing launchers prefer bundled `deepseek-pc-host` and fall back to PATH
- [x] Mobile cockpit chrome: live API/PC/workspace status and dynamic nav badges
- [ ] Durable task artifacts/logs and PC-running-task synchronization
- [ ] Runtime HTTP/SSE API
- [ ] PC-host release package includes matching host binary / explicit installer
- [ ] Real Android visual verification through Dioxus CLI and device/emulator
- [ ] Release packaging and troubleshooting docs
