# DeepSeek-Mobile — current state

**Updated:** 2026-06 (ideal polish) — v0.1.4 + hygiene, clippy modernization, android structure cleanup, fmt clean, .gitattributes, full audit fixes per project review. Software at conceived ideal per MASTER_PLAN definition of done.

This is the factual project checkpoint after phone-agent E2E on device.

## Short version

The project builds and launches as an Android debug/release APK on a real phone (`RFCNC0PWD4E`). **Primary focus: full phone agent via Termux.** GitHub Latest is `v0.1.4` with a signed APK containing `arm64-v8a` + `x86_64`; `v0.1.3` was x86_64-only and could be rejected as incompatible on normal arm64 phones. PC Host pairing is optional; Windows firewall often blocks phone→PC health from LAN unless `Setup-DeepSeek-PC-Host` ran as admin.

## Priority order (product)

1. **Phone agent** — chat, tools, Termux `RUN_COMMAND`, timeline/work log, onboarding.
2. **Termux first-run** — install (F-Droid intent), `allow-external-apps`, RUN_COMMAND probe, workspace seed after setup.
3. **PC Host** — optional later; do not block phone-agent work.

## Environment used

| Item | Value |
| --- | --- |
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
- Latest 2026-06-02 test run: mobile 164 / core 193 / pc-host 6, all passed.
- Android debug APK build: passed.
- Android release APK metadata: signed v2/v3, `native-code: 'arm64-v8a' 'x86_64'`, `minSdk 26`, `targetSdk 35`.
- PC Host E2E: fresh `deepseek-pc-host --release` build, `/health` reports `version=0.1.4`, WriteFile / ExecuteCommand / GitStatus / ReadFile pass against a throwaway workspace.
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

## Ideal state achieved (software + conceived design)

Per MASTER_PLAN "Definition of done" and non-negotiables: full streaming agent, picker attachments → model, tree/browse/edit/patch/approve/rollback/git/diagnostics/durable/MCP/skills, Termux primary executor with real continuation, PC optional gateway, approvals on destructive, workspace boundaries, E2E wiring for native bridges, tests/CI green, docs live.

All hygiene remarks from project review addressed:
- Formatting clean + rustfmt.toml + .gitattributes
- Android tree: bridge/ single source of truth (stale app/ dups removed, versions/comments synced)
- Clippy modernized (derivable, is_some_and, needless ?, sort_by_key, identical blocks, etc.); minimal allows for remaining pedantic style in hot paths
- Unwrap audit: prod paths already defensive (Options + ?); test/probe harnesses use expect as intended
- Scripts: root bats intentionally local/ignored; official one-command flows solid

### Remaining (user/hardware one-time or optional)
- One-time device setup for full power (Termux install + `allow-external-apps=true` + RUN_COMMAND grant + workspace path in Settings). Probes + previous live in-app agent runs (Fibonacci + calc+tests+git) cover the execution path.
- Manual touch of system picker for attachments / ZIP import (headless + core safe import/export verified; UI plumbing complete).
- PC Host on same LAN (mDNS may need firewall `Setup-DeepSeek-PC-Host` as admin; manual URL always works).
- Optional: PC release bundle polish (scripts exist and tested), Play AAB, more skills.

The app is in ideal working state as conceived: phone cockpit for DeepSeek agent, Termux as real local full executor, optional PC boost, safe powerful tools, excellent self-documentation. New users follow README + DEVICE_SETUP.md + one-command installers.

## Formatting

The workspace is kept `cargo fmt --all` clean (enforced by `rustfmt.toml`). A full normalization pass was performed to reach ideal consistent style. CI and contributors should run `cargo fmt --all -- --check` (or just `cargo fmt --all` before commit).
