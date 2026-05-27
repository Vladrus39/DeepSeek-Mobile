# DeepSeek-Mobile — current project status

**Updated:** 2026-05-28

Canonical checkpoint: [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md).

## Overall state

DeepSeek-Mobile has a working Rust core, mobile cockpit UI, Android bridge layer and optional PC Host. The important status change in this checkpoint: **the Dioxus Android APK now builds, installs and launches on a physical phone**.

The project currently supports:

- streamed DeepSeek turns with reasoning deltas;
- persisted sessions, settings, runtime state and approvals;
- approval continuation after tool execution;
- file tools, shell/git/web/GitHub tools, snapshots and diagnostics;
- `apply_patch` with operation batches and unified diff input;
- PC Host gateway for files, shell, git, diagnostics, snapshots, terminal and tasks;
- Termux execution contract through native Android `RUN_COMMAND` bridge;
- mobile panels for chat, approvals, files, snapshots, diagnostics, PC Host, terminal, Git, tasks, MCP, skills and settings;
- project ZIP import/export helpers with path traversal protection;
- Android bridge module bundled into Dioxus builds;
- custom Android launcher icon and SVG favicon;
- Android app data directory initialization and optional debug `.env` API-key prefill for hardware testing.

## Verified in this checkpoint

| Area | Result |
|---|---|
| Rust workspace check | `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` passes |
| Rust workspace tests | `cargo +stable-x86_64-pc-windows-msvc test --workspace` passes |
| Test count | mobile 140 / core 178 / pc-host 3 |
| Android build | `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` passes |
| Android install/launch | APK installs and launches on Samsung `SM_G781B`, serial `RFCNC0PWD4E` |
| Android UI render | Latest hardware smoke test renders setup screen; API/Agent are ready, Termux path is still pending until saved/configured |
| Android crash buffer | No crash after launch smoke test |
| Android icon | Manifest uses `@mipmap/deepseek_launcher` and `@mipmap/deepseek_launcher_round` |
| Local Android SDK | Isolated under `tools/android/`; no dependency on `D:\Project V` |

## Fixed in this checkpoint

- Dioxus Android library loading: Kotlin bridge now loads `libmain.so` first, fallback `libdeepseek_mobile.so` second.
- JNI package mismatch: Rust exports now match `com.deepseek.mobile.bridge.NativeBridge`.
- Dioxus activity restart crash: custom Android manifest handles `assetsPaths` and full config changes.
- Android OpenSSL crash risk: `reqwest` now uses `rustls-tls`, removing the missing `libssl.so` dependency.
- Android bridge packaging: `manganis` metadata includes `android/bridge` in the Dioxus-generated Gradle project.
- Android icon/favicon: adaptive launcher resources and SVG favicon were added.
- Android data directory: JNI initializes `<filesDir>/deepseek-mobile/` before Dioxus UI startup.
- Debug API prefill: Android debug builds can prefill onboarding from `.env`; release builds intentionally ignore it.
- Runtime panic fix: MCP tool loading no longer blocks an active Tokio runtime during initial render.
- SDK repo hygiene: local SDK licenses/cache files are ignored by git.
- Android persistent storage: `NativeBridge.initMobileDataDir` → `<filesDir>/deepseek-mobile/` (config, secrets, runtime, workspace).
- Setup/onboarding: pre-filled debug API key when available, RU/EN toggle, one-screen API + Termux path flow, **sandbox-only** fallback, Agent mode default after setup.

See **`docs/DEVICE_SETUP.md`** for phone checklist (Termux, smoke tests, ADB).

## Implemented but still needs manual end-to-end verification

| Area | Current reality | Remaining verification |
|---|---|---|
| Android document picker | Kotlin bridge + Rust callback routing exist | Pick real file on phone and confirm chat attachment ingestion |
| Project import ZIP | Core helper + Files UI + picker purpose routing exist | Import a real ZIP through Android picker and confirm Files refresh |
| Project export/share | Core ZIP export + native share command exist | Export from phone workspace and confirm Android share sheet receives ZIP |
| Termux executor | RUN_COMMAND intent, callback parser, Rust continuation path exist | Grant permission, set `allow-external-apps=true`, run `pwd`, confirm stdout/stderr/exit code continuation |
| PC Host discovery | mDNS discovery bridge and PC Host route persistence exist | Verify on a real LAN with a running `deepseek-pc-host` |
| UI polish | Android cockpit/onboarding render path works | Walk through all panels on small/large phone widths |

## Priority (2026-05-28)

1. **Phone agent (Termux)** — primary; E2E chat + shell verified on device.
2. Release signing and signed APK/AAB.
3. **PC Host** — deferred: LAN pairing/mDNS after phone path is stable.
4. MCP stdio session reuse; UI polish.

## Highest-value remaining work

1. Remaining manual checklist (picker/ZIP where not preview-disabled).
2. Add release signing and build signed APK/AAB.
3. PC Host pairing on real LAN (later phase).
4. Finish long-lived MCP stdio session reuse.
5. Run final UI/touch polish after full device-flow testing.

## GitHub CI note

The GitHub Actions Rust workspace job installs the Linux GTK/WebKit/pkg-config dependencies needed by the Dioxus mobile crate before `cargo check` and `cargo test`. If GitHub reports a Rust environment failure, inspect the Rust workspace job logs first; locally the MSVC workspace check/test pass.
