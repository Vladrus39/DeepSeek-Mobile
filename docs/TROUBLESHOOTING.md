# DeepSeek-Mobile troubleshooting

## Build and development

### `cargo check` fails on Windows with GNU toolchain

Use the MSVC toolchain for local verification:

```powershell
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace
```

### Android build command

Activate the project-local Android environment first:

```powershell
. .\tools\android\env.ps1
```

Build the debug APK for the connected phone:

```powershell
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```

Use `--android`; do not use the old `dx build android` form for this project.

### GitHub Actions Rust workspace failure

The CI job must install Linux native dependencies needed by the Dioxus mobile crate:

- `pkg-config`
- `libglib2.0-dev`
- `libgtk-3-dev`
- `libwebkit2gtk-4.1-dev`
- `libayatana-appindicator3-dev`
- `libxdo-dev`

The repository workflow already includes these. If GitHub still reports a Rust environment failure, open the `Rust workspace` job log and check whether the failure is dependency installation, compile error, or test failure.

### Android crash: `libssl.so` missing

Cause: an Android dependency tries to use OpenSSL shared libraries that are not packaged in the APK.

Fix used in this project:

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "stream", "rustls-tls"] }
```

### Android crash: `libdeepseek_mobile.so` not found

Cause: Dioxus packages the Rust native activity library as `libmain.so`, not `libdeepseek_mobile.so`.

Fix: `android/bridge/.../NativeBridge.kt` loads `main` first and only then falls back to `deepseek_mobile`.

### Android crash after first frame: destroyed pthread mutex / native restart

Observed symptom: app launches, then Android restarts the Dioxus activity during startup/config change and native code crashes with a destroyed mutex.

Fix: keep the custom Dioxus manifest configured in `dioxus.toml`:

```toml
[application]
android_manifest = "../../android/AndroidManifest.xml"
```

The activity must handle the full config-change set, including `assetsPaths`.

### `cargo fmt --all --check` fails

The workspace currently has pre-existing formatting differences unrelated to this Android checkpoint. For targeted verification of touched Rust files, use:

```powershell
rustfmt --edition 2021 --config skip_children=true --check .\crates\mobile\src\android_plugin.rs .\crates\mobile\src\host_loop.rs .\crates\mobile\src\jni_bridge.rs .\crates\mobile\src\lib.rs
```

Only run `cargo fmt --all` when intentionally accepting a broad formatting-only diff.

## Android host integration

### Native commands stay queued

Symptom: timeline shows `Android host action queued: {...}` but picker/Termux/PC discovery never runs.

Checklist:

1. Build through Dioxus with the bridge module bundled:
   ```powershell
   . .\tools\android\env.ps1
   dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
   ```
2. Confirm `crates/mobile/src/android_plugin.rs` still references `../../android/bridge` through `manganis::ffi`.
3. Confirm `android/MainActivity.kt` creates `DeepSeekMobileHostCoordinator` and polls it.
4. Capture logcat and search for `DeepSeekMobile` / `NativeBridge`.

### Termux commands fail immediately

Checklist:

- Termux is installed.
- `com.termux.permission.RUN_COMMAND` is granted to DeepSeek-Mobile.
- `allow-external-apps=true` is set in `~/.termux/termux.properties`.
- Settings → Termux workspace path is an absolute path under `/data/data/com.termux/files/home/...`.
- Test with a safe command first, for example `pwd`.

## PC Host

### Phone cannot reach PC Host

1. Confirm `deepseek-pc-host` is running on the PC.
2. Open the pairing ZIP launcher or start the host manually with the workspace root and token.
3. Verify firewall rules for the bind port, default `8787`.
4. Use the PC Host panel → scan/retry route and check endpoint health rows.

### PC tasks do not update live

The Tasks panel subscribes to `stream_task_events` over SSE. If SSE is blocked by a proxy, use **Refresh** to reconcile through `list_tasks`.

## Settings and secrets

API keys and GitHub tokens are stored in encrypted `secrets.enc` with a per-device `device.key` under `.deepseek-mobile/`. If decryption fails after copying data directories between machines, delete `device.key` and `secrets.enc`, then re-enter secrets in Settings.

## MCP servers

- HTTP/SSE servers: use **Connect** in the MCP panel to run `tools/list`.
- Stdio servers: require a host OS process; long-lived stdio session reuse is still a remaining v1 item.
