# DeepSeek-Mobile — active progress

**Current session:** 2026-05-26 — Android device launch stabilization, icon/favicon, documentation sync.

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
