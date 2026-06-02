# DeepSeek-Mobile — roadmap

**Updated:** 2026-06 (ideal polish) — Software complete to conceived vision. Remaining items are one-time user hardware setup or optional packaging. See CURRENT_STATE.md "Ideal state achieved".

**Updated:** 2026-05-29 (prior)

## Phase 0 — Foundation ✅

- [x] Cargo workspace with `core`, `mobile`, and `pc-host`.
- [x] CI/local build path for `cargo check` and `cargo test`.
- [x] Runtime persistence and approval continuation.
- [x] Persisted settings used by real turns.

## Phase 1 — Core tool parity ✅

- [x] File ops, shell, git, web, GitHub, snapshots, `apply_patch`.
- [x] `apply_patch` operation batches.
- [x] `apply_patch` unified-diff compatibility.
- [x] Capability and approval routing.
- [x] Post-edit diagnostics hooks.
- [x] `ModelRouter` in real turn selection.
- [x] `ContextManager` in real prompt lifecycle.
- [x] `auto_commit_and_push` invoked by the engine when enabled.

## Phase 2 — Snapshots & rollback ✅

- [x] Snapshot service.
- [x] Pre-tool snapshots.
- [x] Post-turn snapshots.
- [x] Restore UI and pruning.
- [x] PC-gateway snapshot path for remote workspaces.

## Phase 3 — PC gateway & execution 🔄

- [x] HTTP host, auth, security policy, path protection.
- [x] File/git/command operations.
- [x] Streaming commands.
- [x] Pairing ZIP, mDNS discovery, endpoint health/failover.
- [x] Pairing persists the active PC workspace into runtime config.
- [x] Rust/TypeScript/Python diagnostics in PC-host.
- [x] Terminal sessions on PC-host.
- [x] Terminal UI-state persistence across app restarts.
- [x] PC-host background task start/stop/list RPCs.
- [x] Runtime task HTTP API and SSE task events.
- [ ] Dev-server lifecycle.
- [ ] PC-host release bundle and autostart/service installer.

## Phase 4 — Mobile UI ✅ / polish pending

- [x] Chat, approvals, snapshots, diagnostics, settings, onboarding.
- [x] PC Host pairing surface.
- [x] Files tree + preview.
- [x] Real pending/project diffs.
- [x] Remote-aware file browsing for active PC workspace.
- [x] Terminal panel.
- [x] Git panel with real operations.
- [x] Durable task manager panel.
- [x] MCP and Skills panels.
- [x] Android first-screen smoke render on physical phone.
- [ ] Full phone walkthrough for all panels and touch ergonomics.

## Phase 5 — Android & local execution 🔄

- [x] Native bridge contracts for picker, discovery, terminal, share.
- [x] Termux `RUN_COMMAND` bridge contract.
- [x] Termux `exec_shell` native request metadata and mobile queue extraction.
- [x] Rust/mobile Termux result continuation back into the model.
- [x] Termux workspace selector.
- [x] Android import/export UI completion.
- [x] Dioxus Android host adapter: `MainActivity`, JNI bridge, Kotlin coordinator.
- [x] Dioxus Android bridge packaging through `manganis`.
- [x] Physical-device debug APK build/install/launch smoke test.
- [x] Android adaptive launcher icon and SVG favicon.
- [x] Android app-private data directory initialization.
- [x] Optional debug `.env` API prefill for hardware testing.
- [x] MCP startup panic removed from main render path.
- [ ] Device verification of document picker chat attachments (no automated probe yet).
- [x] Device verification of Import ZIP (`device-e2e-zip-import.ps1` PASS 2026-05-29).
- [x] Device verification of Export ZIP/native share (`device-e2e-zip-export.ps1` PASS 2026-05-29).
- [x] Device verification of Termux `RUN_COMMAND` result continuation (`device-termux-pwd-probe.ps1` PASS 2026-05-29).
- [x] Device/LAN PC Host E2E with manual URL fallback (`device-e2e-pc-host.ps1` PASS 2026-05-29; mDNS often blocked on Windows LAN).

## Phase 6 — Product completion 🔄

- [x] Diagnostics injected into the next model turn.
- [x] Durable background task records and task manager UI.
- [x] MCP/skills config and UI layer.
- [x] PC pairing launchers prefer bundled `deepseek-pc-host` and fall back to PATH.
- [x] Mobile cockpit chrome: live API/PC/workspace status and dynamic nav badges.
- [x] Durable task artifacts/logs.
- [x] Runtime HTTP task API.
- [x] PC-running-task synchronization/reconciliation.
- [x] Runtime SSE/live event streaming.
- [x] MCP stdio session reuse (cached child per server, reconnect on invoke failure, panel disconnect).
- [x] External MCP tool execution behind approvals (MCP proxy requires approval; invoke gated by mcp.json server/tool registry).
- [x] Signed release APK build (`android/keystore.properties`, `scripts/build-release-apk.ps1`).
- [x] GitHub Releases publish script and tag workflow (`scripts/publish-github-release.ps1`, `.github/workflows/release.yml`).
- [x] In-app update check + APK download/install (Settings → App update).
- [x] Install/update docs (`docs/INSTALL_UPDATE.md`, `RELEASE_NOTES.md`).
- [ ] PC-host release package with matching host binary.
