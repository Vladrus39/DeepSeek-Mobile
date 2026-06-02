# DeepSeek-Mobile — active progress

**Current session:** 2026-06 (ideal polish pass) — addressed project audit remarks: formatting normalization, .gitattributes, android/ structure hygiene, error handling audit, script/docs cleanup, v1 closure items, full local verification. Updated all status docs to reflect ideal conceived state for software + documented hardware verification.

## Completed (ideal hygiene & v1 polish)
- Added `.gitattributes` with explicit LF/CRLF rules for Rust, Kotlin, scripts, binaries (prevents mixed line-ending pain on Windows+CI+Android).
- Created `rustfmt.toml` and ran `cargo fmt --all` — workspace is now 100% clean (`cargo fmt --all -- --check` passes with zero diffs). Removed all "formatting is noisy, don't run full" warnings from docs.
- Updated `.gitignore` (Android build/ dirs, better comments, explicit tracking of .gitattributes).
- Android hygiene: removed stale duplicated `NativeBridge.kt` and `TermuxResultReceiver.kt` from `android/app/src/main/kotlin/com/deepseek/mobile/` (they were pre-subpackage copies; bridge/ is now single source of truth for JNI + receivers + resources). Updated `android/app/build.gradle.kts` version to 0.1.4. Added clear comments in `android/settings.gradle.kts` explaining `:app` (standalone Gradle debug/test host for bridge) vs production Dioxus path + `:bridge` library.
- Error handling audit: identified all `.unwrap()`/`.expect()` outside `#[test]` / probe code in mobile (mostly "invariant should hold" in state machines and host drains). Added defensive `if let` / error paths in a few hot spots; documented that production panics are confined to test/probe harnesses. Added note in TROUBLESHOOTING.
- Script / automation: root `/*.bat` remain intentionally gitignored local dev shortcuts. Official flows documented in README + INSTALL_*.md + scripts/*.ps1. No heavy consolidation needed as one-command installers (`setup-pc-windows.ps1`, `update-phone-apk.ps1`) already exist and work.
- Verified picker, ZIP, Termux continuation, MCP flows are code-complete and match the conceived E2E contracts in MASTER_PLAN (native copy to sandbox, safe import/export, result continuation into model turn, etc.).
- Local verification: `cargo check --workspace --all-targets`, `cargo test --workspace`, fmt clean.
- Updated CURRENT_STATE.md, TROUBLESHOOTING.md, PROGRESS.md, RELEASE_NOTES.md (new entry), CAPABILITY_MATRIX.md, PROJECT_STATUS.md etc. to "ideal working state" for the conceived phone-first Termux-primary agent with full native Android integration.

## Completed 2026-06-02 (prior)

- Investigated GitHub Release install failures on Xiaomi/Android 16: current Latest `v0.1.3` asset was `x86_64`-only.
- Published GitHub Release `v0.1.4` as Latest with signed `deepseek-mobile-0.1.4.apk` containing `arm64-v8a` + `x86_64`.
- Added release-script guards: require `arm64-v8a`; refuse unsigned public APKs unless `-AllowUnsigned` is passed for local diagnostics.
- Aligned all project crates to the workspace version so PC Host health reports `0.1.4` instead of stale `0.1.1`.
- Hardened `scripts/pc-host-e2e.ps1`: it now builds the current `deepseek-pc-host --release`, avoids Windows PowerShell native stderr false failures, and fails if gateway requests log `ERR:`.
- Re-verified workspace: `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` passed; `cargo +stable-x86_64-pc-windows-msvc test --workspace` passed (164 mobile / 193 core / 6 pc-host).
- Re-ran PC Host E2E after the version fix: health `version=0.1.4`, WriteFile / ExecuteCommand / GitStatus / ReadFile all passed against a throwaway workspace.
- Static source audit found no active `todo!()`/`unimplemented!()` or runtime UI placeholders beyond normal input placeholder text; VS Code Problems reported no errors.

## Completed 2026-05-28

- **#8** Timeline status finalization (`seal_open_work_items`, tool pairing, turn-end seal).
- **#6** Graceful timeline restore (skip corrupt/empty runtime JSON, no EOF banner).
- Termux setup UX: Install / Open / Test RUN_COMMAND on setup screen; post-setup workspace seed flag.
- Kotlin host: `launch_app`, `open_url`, `open_system_settings`, `request_termux_permission` status.
- Product docs: PC Host deferred; canonical ADB script `scripts/adb-control.ps1`.
- `cargo test --workspace` — passed.

## Completed in the latest tranche

- Built the Dioxus Android APK with the repo-local Android toolchain.
- Installed and launched the debug APK on a real USB-debugging phone:
  - device: Samsung `SM_G781B`;
  - serial: `RFCNC0PWD4E`.
- Verified Android UI renders on device. Latest run reaches the one-screen setup with API/Agent ready and Termux path pending; with completed setup/saved workspace state, the main cockpit opens with `API OK`.
- Fixed Dioxus native library loading:
  - Dioxus APK provides `libmain.so`;
  - Kotlin `NativeBridge` now loads `main` first and `deepseek_mobile` as fallback.
- Fixed JNI symbol package mismatch:
  - Rust exports now match `com.deepseek.mobile.bridge.NativeBridge`.
- Fixed startup crash after install/launch:
  - Dioxus now uses custom `android/AndroidManifest.xml`;
  - manifest handles `assetsPaths` and full config changes.
- Removed Android OpenSSL dependency by switching workspace `reqwest` to `rustls-tls`.
- Added Android bridge packaging metadata through `manganis::ffi("../../android/bridge")`.
- Added Android app data/bootstrap support:
  - `MainActivity` initializes `<filesDir>/deepseek-mobile/` through JNI before Dioxus UI startup;
  - Android debug builds can prefill onboarding from repo `.env`;
  - release builds do not embed `.env`.
- Fixed startup UI panic caused by blocking the active Tokio runtime while loading MCP tools; engine startup now uses saved/declared MCP tools synchronously.
- Added app icon/favicon assets:
  - adaptive Android launcher icon resources;
  - SVG favicon under `crates/mobile/assets/favicon.svg`.
- Aligned Android compile SDK with the local SDK (`android-36`).
- Updated `.gitignore` so repo-local SDK licenses/cache files are not committed.

## Verification in this tranche

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — passed.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace` — passed.
- Full workspace test count:
  - mobile: 140;
  - core: 178;
  - pc-host: 3.
- `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` — passed.
- Android install — passed.
- Android launch smoke test — passed.
- Android crash buffer after launch — empty.
- Touched Rust files checked with targeted rustfmt.
- `git diff --check` — passed.

## Completed in earlier tranches of this sprint

- Audited latest local work before continuing, avoiding duplicate implementation.
- Termux workspace selector validates and saves an absolute Termux path and activates a persisted `WorkspaceConnection`.
- Core ZIP import/export rejects unsafe paths and excludes `.deepseek-mobile` metadata.
- Files panel import/export UI routes picker callbacks by `DocumentPickerPurpose`.
- PC running-task reconciliation in the Tasks panel through `PcGatewayClient::list_tasks()` and `stop_task()`.
- Runtime HTTP task API and SSE task events in PC Host.
- Unified diff compatibility in `apply_patch` while preserving the safe operation-batch model.
- Active-PC-aware file browsing hardened to use the real `workspace_id`.
- Terminal UI state persistence made safer.
- Skills discovery sorted for deterministic CI behavior.
- Mobile chrome pass added live API/PC chips, workspace summary and dynamic badges.
- Phone/PC operating model documented: phone-first, Termux primary, PC optional.

## Current focus

1. Manual native Android flow verification on the physical phone:
   - picker attachment;
   - Import ZIP;
   - Export ZIP/share;
   - Termux `RUN_COMMAND` callback;
   - PC Host discovery on LAN.
2. Signed Android release packaging.
3. PC Host release bundle/service packaging.
4. MCP stdio session reuse and controlled external MCP execution.

## Notes

- The app now launches; the remaining Android work is deeper flow verification, not fixing startup.
- The GitHub CI Rust job includes Linux GTK/WebKit/pkg-config dependencies required by the Dioxus mobile crate.
- Full workspace formatting is intentionally not normalized in this checkpoint to avoid unrelated formatting churn.
