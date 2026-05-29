# DeepSeek-Mobile — current project status

**Updated:** 2026-05-29

Canonical checkpoint: [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md). Device evidence matrix: [`docs/DEVICE_E2E_RESULTS.md`](docs/DEVICE_E2E_RESULTS.md). Tool evidence matrix: [`docs/TOOL_AUDIT.md`](docs/TOOL_AUDIT.md).

## Overall state

DeepSeek-Mobile has a working Rust core, mobile cockpit UI, Android bridge layer, **primary Termux phone-agent path**, and optional PC Host. The Android debug APK builds, installs, launches, and renders on a physical phone. The product direction remains phone-first: Termux is the default full-agent executor; PC Host is an optional boost for large repos or desktop-only toolchains.

The project currently supports:

- streamed DeepSeek turns with reasoning deltas;
- persisted sessions, settings, runtime state and approvals;
- approval continuation after tool execution;
- file tools, shell/git/web/GitHub tools, snapshots and diagnostics;
- `apply_patch` with operation batches and unified diff input;
- Termux execution contract through native Android `RUN_COMMAND` bridge;
- Termux callback continuation from stdout/stderr/exit code back into the model turn;
- PC Host gateway for files, shell, git, diagnostics, snapshots, terminal and tasks;
- mobile panels for chat, approvals, files, snapshots, diagnostics, PC Host, terminal, Git, tasks, MCP, skills and settings;
- project ZIP import/export helpers with path traversal protection;
- Android bridge module bundled into Dioxus builds;
- custom Android launcher icon and SVG favicon;
- Android app data directory initialization and optional debug `.env` API-key prefill for hardware testing;
- optional GitHub Releases APK update path when signed release assets are published.

## Verified in this checkpoint

| Area | Result |
|---|---|
| Rust workspace check | `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` passes |
| Rust workspace tests | `cargo +stable-x86_64-pc-windows-msvc test --workspace` passes |
| Test count | mobile 140 / core 178 / pc-host 3 in the recorded checkpoint |
| Android build | `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` passes |
| Android install/launch | APK installs and launches on Samsung `SM_G781B`, serial `RFCNC0PWD4E` |
| Android UI render | Setup/cockpit render path works; latest smoke reaches setup with API/Agent ready and Termux path pending until configured |
| Android crash buffer | No crash after launch smoke test |
| Termux agent path | API probe, agent probe, Termux calibration, `pwd`, file create/edit/copy/delete workflows pass in automated device probes |
| Sandbox tools | `workspace_overview`, `apply_patch`, `read_file` pass in app sandbox probe |
| ZIP transfer | Export/share headless PASS; import headless PASS; system picker UI still manual |
| MCP demo | `mcp__demo__echo` PASS through LAN JSON-RPC demo server |
| PC Host health | `http://127.0.0.1:8787/health` PASS when host is running |
| PC Host discovery | mDNS is implemented but requires same LAN/subnet and firewall; manual LAN URL fallback is documented |
| Android icon | Manifest uses `@mipmap/deepseek_launcher` and `@mipmap/deepseek_launcher_round` |
| Local Android SDK | Isolated under `tools/android/`; no dependency on `D:\Project V` |

## Fixed in recent checkpoints

- Dioxus Android library loading: Kotlin bridge now loads `libmain.so` first, fallback `libdeepseek_mobile.so` second.
- JNI package mismatch: Rust exports now match `com.deepseek.mobile.bridge.NativeBridge`.
- Dioxus activity restart crash: custom Android manifest handles `assetsPaths` and full config changes.
- Android OpenSSL crash risk: `reqwest` now uses rustls, removing the missing `libssl.so` dependency.
- Android bridge packaging: `manganis` metadata includes `android/bridge` in the Dioxus-generated Gradle project.
- Android icon/favicon: adaptive launcher resources and SVG favicon were added.
- Android data directory: JNI initializes `<filesDir>/deepseek-mobile/` before Dioxus UI startup.
- Debug API prefill: Android debug builds can prefill onboarding from `.env`; release builds intentionally ignore it.
- Runtime panic fix: MCP tool loading no longer blocks an active Tokio runtime during initial render.
- SDK repo hygiene: local SDK licenses/cache files are ignored by git.
- Setup/onboarding: pre-filled debug API key when available, RU/EN toggle, one-screen API + Termux path flow, sandbox-only fallback, Agent mode default after setup.
- Chat/work-log fixes: scroll-to-latest, sealed stale work items, duplicate continuation event removal, loading timeout guard.
- PC Host mDNS/firewall support: host advertises `_deepseek-pc-gateway._tcp.local`; Windows helper opens TCP/mDNS rules.
- Windows release workflow now pins Dioxus CLI to the verified `0.7.9` toolchain.

## Implemented but still needs manual end-to-end verification

| Area | Current reality | Remaining verification |
|---|---|---|
| Android document picker | Kotlin bridge + Rust callback routing exist | Pick one real file on phone and confirm chat attachment ingestion |
| Project import ZIP | Core helper + Files UI + picker purpose routing exist; headless import PASS | Import one real ZIP through Android system picker and confirm Files refresh |
| Project export/share | Core ZIP export + native share command exist; headless export/share PASS | Optional chooser UX confirmation |
| Termux executor | Automated probes pass when configured | Manual user-chat flow: ask for `pwd`, approve, confirm continuation text |
| PC Host discovery | Host, mDNS, manual URL fallback implemented | Same-subnet LAN test with phone and PC both on `192.168.1.x` or equivalent |
| UI polish | Onboarding/cockpit render path works | Walk every bottom-nav/drawer panel on small/large phone widths |
| Release APK | Build/publish scripts and release workflow exist | Configure signing secrets/keystore, tag release, verify asset installs and in-app updater sees it |

## Priority

1. **Phone agent (Termux)** — primary; keep hardening manual chat + shell flows.
2. Release signing and signed APK/AAB / GitHub Release asset.
3. PC Host bundle/service polish after phone path remains stable.
4. MCP stdio session reuse and external execution hardening.
5. UI/touch sweep across all panels.

## GitHub CI note

The GitHub Actions Rust workspace job installs Linux GTK/WebKit/pkg-config dependencies needed by the Dioxus mobile crate before `cargo check` and `cargo test`. If GitHub reports a Rust environment failure, inspect the Rust workspace job logs first; local MSVC workspace check/test passed in the recorded checkpoint.
