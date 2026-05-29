# DeepSeek-Mobile — current state

**Updated:** 2026-05-30 (v0.1.2)

This is the factual project checkpoint after phone-agent E2E on device.

## Short version

The project builds and launches as an Android debug/release APK on a real phone (`RFCNC0PWD4E`). **Primary focus: full phone agent via Termux.** v0.1.2 adds chat snapshot rollback, expandable work log with per-step tool details, open project folder from work log, and snapshots panel refresh. PC Host pairing is optional; Windows firewall often blocks phone→PC health from LAN unless `Setup-DeepSeek-PC-Host` ran as admin.

## Priority order (product)

1. **Phone agent** — chat, tools, Termux `RUN_COMMAND`, timeline/work log, onboarding.
2. **Termux first-run** — install (F-Droid intent), `allow-external-apps`, RUN_COMMAND probe, workspace seed after setup.
3. **PC Host** — optional later; do not block phone-agent work.

## Environment used

| Item | Value |
|---|---|
| Workspace | `C:\Users\vladi\Desktop\DeepSeek-Mobile` |
| Android device | Samsung `SM_G781B` |
| ADB serial | `RFCNC0PWD4E` |
| Dioxus CLI | `dx 0.7.9` |
| Android SDK | `tools/android/sdk` |
| Android NDK | `tools/android/sdk/ndk/26.1.10909125` |
| Rust Android targets | Installed |
| Gradle wrapper cache | Present locally |

## Verified commands

```powershell
. .\tools\android\env.ps1
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```

Result:

- Rust workspace check: passed.
- Rust workspace tests: passed.
- Android debug APK build: passed.
- APK install on connected phone: passed.
- Launch smoke test: passed.
- Android UI rendered: passed. Latest hardware smoke test reaches the setup screen with API/Agent ready and Termux path pending; with completed setup the main cockpit opens with `API OK`.
- Crash buffer after launch: empty.

Latest known Android APK path:

```text
target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
```

Latest local evidence files:

```text
target/dx-android-build-final-publish3.log
target/android-logcat-publish.txt
target/android-crash-publish.txt
target/android-device-screenshot-publish.png
```

These files are local build artifacts and are intentionally not committed.

## Android startup fixes completed

1. **Native library loading**
   - Dioxus packages the Rust native activity library as `libmain.so`.
   - `android/bridge/.../NativeBridge.kt` now loads `main` first and keeps `deepseek_mobile` as fallback.

2. **JNI package alignment**
   - Rust JNI exports now use `Java_com_deepseek_mobile_bridge_NativeBridge_*`.
   - This matches the Kotlin bridge package `com.deepseek.mobile.bridge`.

3. **Dioxus activity restart crash**
   - Custom `android/AndroidManifest.xml` is now used by `dioxus.toml`.
   - The activity handles `assetsPaths` and the full config-change set, preventing the observed destroyed-mutex crash during startup/config change.

4. **OpenSSL dependency removal**
   - Workspace `reqwest` uses `default-features = false` with `rustls-tls`.
   - Android APK no longer needs missing `libssl.so`.

5. **Bridge packaging**
   - `crates/mobile/src/android_plugin.rs` declares the Kotlin bridge with `manganis::ffi`.
   - `dx build --android` now copies `android/bridge` into the generated Android project.

6. **Android data directory and debug bootstrap**
   - `MainActivity` initializes `<filesDir>/deepseek-mobile/` through JNI before Dioxus UI startup.
   - Android debug builds can prefill onboarding from repo `.env` for faster device testing.
   - Release builds do not embed `.env`.

7. **Runtime startup panic fix**
   - MCP tool loading no longer calls `Handle::block_on` inside an active Tokio runtime.
   - Engine startup uses saved/declared MCP tools synchronously; explicit MCP connection remains a UI action.

8. **Icon/favicon**
   - Android adaptive icon resources were added under `android/bridge/src/main/res`.
   - The Dioxus manifest uses `@mipmap/deepseek_launcher` and `@mipmap/deepseek_launcher_round`.
   - SVG favicon asset added under `crates/mobile/assets/favicon.svg`.

## Device configuration (after APK install)

Follow **`docs/DEVICE_SETUP.md`**:

- Debug APK can prefill the onboarding API key from repo `.env` on rebuild (`crates/mobile/build.rs`, debug only).
- App storage path: `NativeBridge.initMobileDataDir` → `<filesDir>/deepseek-mobile/`.
- Current device state after the latest smoke test: setup screen opens, API/Agent checks pass, Termux path remains pending until the path is saved and Termux external command permission is verified.
- Termux path + `allow-external-apps` required for full TUI-class agent.

## Tool & skills status

- Full built-in tool list and E2E coverage matrix: **`docs/TOOL_AUDIT.md`**
- **21 skills** in `skills-bundle/`; push with `scripts/push-skills-to-device.ps1`
- Automated PASS on device: Termux file/shell/git, MCP echo, ZIP export+import headless (see `docs/DEVICE_E2E_RESULTS.md`)

## What remains

### Native Android end-to-end verification

Run manually on the phone:

- Pick one source/text file through Android picker and confirm it appears as a chat attachment.
- Import one project ZIP through **system picker** (Files → Import ZIP) once — see `docs/ZIP_IMPORT_UI_TEST.md` (headless import PASS via adb).
- Export through UI if you want to confirm chooser UX (headless export+share already PASS).
- Configure Termux:
  - install Termux;
  - grant `com.termux.permission.RUN_COMMAND`;
  - set `allow-external-apps=true` in `~/.termux/termux.properties`;
  - save a valid Termux project path in Settings;
  - run `pwd` and verify stdout/stderr/exit code callback and model continuation.
- **PC Host (phase 4):** `scripts/device-e2e-pc-host.ps1` and `scripts/device-e2e-pc-pairing-bundle.ps1` **PASS** on device `RFCNC0PWD4E` when phone + PC share `192.168.1.x`. mDNS from Windows is often blocked; E2E uses manual LAN URL fallback and in-app **Connect manually** (`docs/PC_HOST_E2E.md`).

### Release work

- Add release signing config outside the repo.
- Produce signed APK/AAB.
- Add release install notes.
- Package matching `deepseek-pc-host` binaries for pairing/release bundles.
- Add optional PC Host service/autostart installer.

### Agent/runtime closure

- Long-lived MCP stdio session reuse.
- External MCP tool execution behind approval/workspace boundaries.
- Optional symbol search/LSP diagnostics later.

## Known formatting note

`cargo fmt --all --check` is currently noisy because of pre-existing formatting differences across the workspace. Touched Rust files in this checkpoint were checked with `rustfmt --edition 2021 --config skip_children=true --check`. Do not run a full workspace format pass unless intentionally accepting a large formatting-only diff.
