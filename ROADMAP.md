# DeepSeek-Mobile Roadmap

## Vision

DeepSeek-Mobile is a **full coding agent on Android** in a DeepSeek-TUI-like workflow. The phone is the cockpit, the model runs through the configured API, the Rust core manages turns/tools/approvals, and execution happens through one of three backends:

1. **Termux workspace** — primary phone-native full-agent path.
2. **Local Android workspace** — safe sandbox/lite mode.
3. **PC Host** — optional workstation backend for huge repos or desktop-only toolchains.

Current factual checkpoint: `docs/CURRENT_STATE.md`.

## Current Android checkpoint — 2026-05-26

- [x] Project-local Android SDK/NDK environment.
- [x] Dioxus CLI `dx 0.7.9` available.
- [x] Rust Android targets installed.
- [x] `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` passes.
- [x] Debug APK installs and launches on Samsung `SM_G781B`.
- [x] Android UI renders: onboarding without a saved key, main cockpit when a key is saved.
- [x] Android launcher icon/favicon assets added.
- [x] Dioxus `libmain.so` loading fixed.
- [x] JNI package mismatch fixed.
- [x] Custom manifest prevents observed startup/config-change crash.
- [ ] Manual picker/import/export/share/Termux/PC-discovery checks.
- [ ] Signed release APK/AAB.

## Phase 0 — Stabilize repository ✅

- [x] Add missing session module.
- [x] CI workflow for `cargo check` + `cargo test` + Android bridge static check.
- [x] Ensure workspace compiles on the intended local toolchain.
- [x] Session persistence JSON file storage.
- [x] Streaming command execution via SSE on PC-host.
- [x] Policy presets for PC-host security.
- [x] Git UI panel in mobile cockpit.
- [x] Architecture/status documents.

## Phase 1 — Real chat core ✅

- [x] Store API key through a mobile-safe settings flow.
- [x] Use full message history.
- [x] Request/response error model.
- [x] DeepSeek provider abstraction.
- [x] Non-streaming and streaming chat implementation.

## Phase 2 — Streaming agent events ✅

- [x] `AgentEvent` enum.
- [x] SSE streaming API client.
- [x] Text/reasoning deltas in mobile UI.
- [x] Persisted event timeline for resume.

## Phase 3 — Workspace and files ✅

- [x] Workspace model with path traversal protection.
- [x] Project import/export ZIP helpers.
- [x] File tree and preview.
- [x] File read/write/edit/apply-patch tools.
- [x] Diff viewer and approval screen.

## Phase 4 — Tool-calling loop ✅

- [x] Tool schemas and JSON contracts.
- [x] Tool specs sent to model.
- [x] Tool call parsing.
- [x] Approval policy execution.
- [x] Tool results returned to model.
- [x] Multi-round follow-up loop.

## Phase 5 — Execution policy ✅

- [x] Plan mode.
- [x] Agent mode.
- [x] YOLO mode.
- [x] Dangerous command blocker.
- [x] Workspace boundary checks.
- [x] Per-tool approval rules.

## Phase 6 — Termux and remote execution 🔄

- [x] Executor trait.
- [x] Local Android file workspace.
- [x] Termux bridge request/callback/continuation path.
- [x] Remote PC-host executor.
- [x] Command output to UI.
- [x] Persist command logs.
- [ ] Real Termux `RUN_COMMAND` hardware verification.

## Phase 7 — Large project support 🔄

- [x] Project index and task detection.
- [x] Large-output routing.
- [x] `workspace_overview` tool.
- [x] `file_summary` tool.
- [x] Diagnostics providers.
- [x] Snapshot/rollback.
- [x] Cost/context tracking.
- [ ] Symbol search hooks.
- [ ] LSP diagnostics through remote or Termux executor.

## Phase 8 — Plugins and integrations 🔄

- [x] MCP config registry and HTTP connect.
- [x] Declared-tools fallback and mobile UI.
- [x] MCP stdio spawn/proxy surfaces in registry/engine.
- [x] GitHub tools and GitHub API client.
- [x] Git operations and Git UI.
- [x] Auto-commit/push helper.
- [x] Durable task queue + PC-host background tasks + SSE events.
- [ ] MCP stdio session reuse + on-device invoke verification.
- [ ] External MCP execution hardening behind approvals.

## Phase 9 — Android packaging 🔄

- [x] Kotlin bridge module.
- [x] Rust `android_host` drain + callback JSON + JNI `NativeBridge`.
- [x] Dioxus `MainActivity`.
- [x] Project-local Android SDK/NDK.
- [x] Debug APK build/install/launch smoke test on real phone.
- [x] Android app icon/favicon.
- [x] Android app-private data directory initialization.
- [x] Optional debug `.env` API prefill for hardware testing.
- [x] MCP startup panic removed from main render path.
- [ ] Manual native flow verification.
- [ ] Signed release APK/AAB and store/release notes.
