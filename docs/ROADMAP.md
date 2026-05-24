# DeepSeek-Mobile — roadmap

**Updated:** 2026-05-25

## Phase 0 — Foundation ✅

- [x] Cargo workspace with `core`, `mobile`, and `pc-host`
- [x] CI/local build path for `cargo check` and `cargo test`
- [x] Runtime persistence and approval continuation
- [x] Persisted settings used by real turns

## Phase 1 — Core tool parity ✅ / partial integration

- [x] File ops, shell, git, web, GitHub, snapshots, `apply_patch`
- [x] Capability and approval routing
- [x] Post-edit diagnostics hooks
- [x] Integrate `ModelRouter` into real turn selection
- [x] Integrate `ContextManager` into real prompt lifecycle
- [x] Invoke `auto_commit_and_push` from the engine when enabled

## Phase 2 — Snapshots & rollback 🔄

- [x] Snapshot service
- [x] Pre-tool snapshots
- [x] Post-turn snapshots
- [x] Restore UI and pruning
- [ ] PC-gateway snapshot path for remote workspaces

## Phase 3 — PC gateway & execution 🔄

- [x] HTTP host, auth, security policy, path protection
- [x] File/git/command operations
- [x] Streaming commands
- [x] Pairing ZIP, mDNS discovery, endpoint health/failover
- [x] Pairing now persists the active PC workspace into runtime config
- [x] Rust/TypeScript/Python diagnostics in PC-host
- [x] Terminal sessions on PC-host
- [ ] Terminal session persistence across app restarts
- [ ] Dev-server lifecycle
- [ ] PC-host autostart/service installer

## Phase 4 — Mobile UI 🔄

- [x] Chat, approvals, snapshots, diagnostics, settings, onboarding
- [x] PC host pairing surface
- [x] Files tree + preview
- [x] Terminal panel
- [x] Git panel surface
- [x] Wire Git panel buttons to real operations
- [ ] Replace illustrative Files diff preview with real pending/project diffs
- [ ] Make file browsing remote-aware when a PC workspace is active

## Phase 5 — Android & local execution 🔄

- [x] Native bridge contracts for picker, discovery, terminal, share
- [ ] Final Android host integration
- [x] Termux RUN_COMMAND bridge contract
- [x] Termux `exec_shell` native request metadata and mobile queue extraction
- [ ] Termux executor lifecycle end-to-end
- [ ] Android import/export completion

## Phase 6 — Product completion

- [x] Diagnostics injected into the next model turn
- [ ] Durable background task model and task manager UI
- [ ] Runtime HTTP/SSE API
- [ ] MCP/plugin/skills layer
- [ ] Release packaging and troubleshooting docs
