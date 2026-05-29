# DeepSeek-Mobile

Mobile-first **DeepSeek Coding Agent** for Android: the phone is the main cockpit, Termux is the primary on-device executor for real projects, and PC Host is optional for large repos or desktop-only toolchains.

Canonical product stance: [`docs/PRODUCT_POSITIONING.md`](docs/PRODUCT_POSITIONING.md).
Current factual checkpoint: [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md).
Real device setup: [`docs/DEVICE_SETUP.md`](docs/DEVICE_SETUP.md) (`.env` debug prefill, Termux path, smoke tests).
Windows PC install/update: [`docs/INSTALL_PC_WINDOWS.md`](docs/INSTALL_PC_WINDOWS.md).

## Current state — 2026-05-29

**Primary goal:** full coding agent on the phone (Termux), like the desktop TUI — **not** PC Host pairing first. A debug Android APK builds, installs, and launches on a real USB-debugging phone (`RFCNC0PWD4E`).

Verified locally:

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — passes.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace` — passes.
- `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` — passes.
- APK installs and launches on Samsung `SM_G781B` / serial `RFCNC0PWD4E`.
- Android UI renders on device. Latest hardware smoke test reaches the setup screen with API/Agent ready and Termux path still pending; with completed setup it opens the main cockpit with `API OK`.
- Custom Android icon/adaptive launcher icon is included.
- **Signed release APK** `dist/deepseek-mobile-0.1.1.apk` (v0.1.1, APK Signature Scheme v4) built via `scripts/build-release-apk.ps1`, published as GitHub Release `v0.1.1`.
- **Live in-app full-agent run on device:** the phone agent created complete Termux projects from chat — a Fibonacci demo and a calculator with assert-based tests — ran them (real differentiated output, e.g. `add(10,5)=15 … divide(10,5)=2.0`), and `git init`/committed each project. Termux's home is app-private (not adb-readable, and Android scoped storage blocks Termux writes to `/sdcard`), so the created files are confirmed through the app's Termux view rather than an external adb dump.

The Android startup issues found during device testing were fixed:

- Dioxus packages the Rust library as `libmain.so`; the bridge now loads `main` first and keeps `deepseek_mobile` as fallback.
- JNI exports now match `com.deepseek.mobile.bridge.NativeBridge`.
- The Android manifest handles `assetsPaths` and other config changes to avoid the native Dioxus activity restart crash.
- `reqwest` uses rustls, so the APK no longer depends on missing Android `libssl.so`.

## How the product is intended to work

| Mode | Purpose |
|---|---|
| **Termux workspace** | Main phone-native full-agent path: shell, git, build, tests in a real Termux project directory. |
| **Local Android sandbox** | Lite mode: chat, attachments, safe file edits, ZIP import/export, snapshots. |
| **PC Host** | Optional workstation backend for very large repos or desktop-specific toolchains. |

PC pairing is not file sync. It grants the phone access to a PC Host workspace. For phone-only work, the app should use Termux or the local sandbox.

## What is implemented

- DeepSeek streaming chat with reasoning deltas.
- Session/runtime persistence and approval continuation.
- File tools, `apply_patch` operation batches and unified diff input.
- Shell/git/web/GitHub tools with approval policy.
- Snapshots before destructive tools and after successful turns.
- Diagnostics for Rust, TypeScript and Python, including model-readable reinjection on the next turn.
- Git panel and engine auto-commit/push lifecycle when enabled.
- PC Host gateway: pairing, discovery, tasks, terminal, SSE events, snapshots and diagnostics.
- Termux bridge contract: queued native command, callback correlation and model continuation path.
- Files panel import/export ZIP helpers with path traversal protection.
- Mobile cockpit UI: chat, approvals, files, snapshots, diagnostics, PC Host, terminal, Git, tasks, MCP, skills and settings.
- Android bridge module bundled into the Dioxus APK through `manganis` metadata.
- Android app data directory initialized under `<filesDir>/deepseek-mobile/`.
- Optional debug `.env` API-key prefill for device testing; release builds do not embed `.env`.
- Android adaptive launcher icon and SVG favicon asset.

## What remains before v1

1. Hardware end-to-end checks for native flows:
   - Android document picker for chat attachments;
   - Files → Import ZIP;
   - Files → Export ZIP and native share;
   - Termux `RUN_COMMAND` permission/result callback with a safe command such as `pwd`;
   - PC Host mDNS discovery and persisted route on a real network.
2. Release packaging — **done** (v0.1.1):
   - release signing config outside the repo (`android/keystore.properties`); ✅
   - signed APK in `dist/` + GitHub Release `v0.1.1`; ✅
   - release notes (`RELEASE_NOTES.md`) and install instructions. ✅
   - remaining/optional: Play Store AAB submission.
3. PC Host packaging:
   - bundled host binaries for pairing ZIP/release package;
   - optional service/autostart installer.
4. MCP closure:
   - long-lived stdio session reuse;
   - external MCP tool execution behind explicit approvals and workspace boundaries.
5. Final UI polish after deeper touch-flow testing on phone.

## Device control (ADB)

Canonical script for screenshots, logcat, install, and taps on a connected phone:

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action InstallLaunch -Serial RFCNC0PWD4E
```

See [`docs/ADB_CONTROL.md`](docs/ADB_CONTROL.md). Avoid `device-*-verify.ps1` while manually testing chat (they `force-stop` the app).

## Quick start

### Windows PC install or update — one command

Open PowerShell as Administrator and run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s
```

The same command is used later to update an existing installation to the latest `main` branch. It keeps the existing `.env` file and does not overwrite local secrets.

Faster update without full tests:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s -SkipTests
```

Alternative clone/update from inside a checkout: [`docs/INSTALL_UPDATE.md`](docs/INSTALL_UPDATE.md) (`scripts/install-windows.ps1`, `scripts/update-windows.ps1`).

### Android APK — build and install on phone (one command)

The debug APK is **not** stored in git. It is built locally and installed over USB. User data on the phone (`files/deepseek-mobile/`) is kept on upgrade (`adb install -r`).

```powershell
cd $HOME\DeepSeek-Mobile
. .\tools\android\env.ps1
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Launch
```

Update git source **and** reinstall the APK:

```powershell
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Pull -Launch
```

APK path after build:

```text
target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
```

There is **no in-app OTA** yet — after every `git pull`, run `update-phone-apk.ps1` again. Full details: [`docs/INSTALL_UPDATE.md`](docs/INSTALL_UPDATE.md), phone setup: [`docs/DEVICE_SETUP.md`](docs/DEVICE_SETUP.md).

Run the desktop UI after setup:

```powershell
cd $HOME\DeepSeek-Mobile
cargo run -p deepseek-mobile
```

Run PC Host after setup (optional, later phase):

```powershell
cd $HOME\DeepSeek-Mobile
$env:DEEPSEEK_PC_HOST_BIND='127.0.0.1:8787'
$env:DEEPSEEK_PC_HOST_WORKSPACE=$PWD
$env:DEEPSEEK_PC_HOST_TOKEN='123456789'
cargo run -p deepseek-pc-host
```

### Android debug APK build

Prefer the one-command installer (build + `adb install -r`):

```powershell
. .\tools\android\env.ps1
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Launch
```

Manual build only:

```powershell
. .\tools\android\env.ps1
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```

Output: `target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk`

### Developer commands

```powershell
git clone https://github.com/Vladrus39/DeepSeek-Mobile.git
cd DeepSeek-Mobile

# Activate repo-local Android SDK/NDK environment.
. .\tools\android\env.ps1

# Rust checks on Windows/MSVC.
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace

# Android debug APK for the connected phone.
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```

## Android toolchain notes

The project uses an isolated Android SDK under `tools/android/`; it does not depend on `D:\Project V`.

See:

- [`tools/android/README.md`](tools/android/README.md)
- [`tools/android/DOWNLOAD_BUDGET.md`](tools/android/DOWNLOAD_BUDGET.md)
- [`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md)
- [`docs/INSTALL_UPDATE.md`](docs/INSTALL_UPDATE.md)

## Main documentation

- [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) — current checkpoint and remaining work.
- [`docs/INSTALL_UPDATE.md`](docs/INSTALL_UPDATE.md) — one-command Windows install/update and phone APK update.
- [`docs/DEVICE_SETUP.md`](docs/DEVICE_SETUP.md) — real phone setup and smoke tests.
- [`docs/ADB_CONTROL.md`](docs/ADB_CONTROL.md) — `scripts/adb-control.ps1` device automation.
- [`docs/INSTALL_PC_WINDOWS.md`](docs/INSTALL_PC_WINDOWS.md) — one-command Windows PC install/update and PC Host startup.
- [`PROJECT_STATUS.md`](PROJECT_STATUS.md) — compact project status.
- [`docs/PROJECT_AUDIT.md`](docs/PROJECT_AUDIT.md) — deeper audit.
- [`docs/PROGRESS.md`](docs/PROGRESS.md) — chronological progress log.
- [`docs/PHONE_PC_OPERATING_MODEL.md`](docs/PHONE_PC_OPERATING_MODEL.md) — phone/PC organization.
- [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) — honest user-facing capabilities.
- [`docs/android_host_integration.md`](docs/android_host_integration.md) — Android native bridge checklist.
- [`docs/UI_STATUS_AND_VERIFICATION.md`](docs/UI_STATUS_AND_VERIFICATION.md) — UI state and visual verification.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — execution roadmap.
- [`docs/MASTER_PLAN.md`](docs/MASTER_PLAN.md) — long-form implementation plan.
