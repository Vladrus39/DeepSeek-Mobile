# Android host integration checklist

**Updated:** 2026-05-26

This document tracks the boundary between the Rust/Dioxus mobile code and the native Android bridge module.

Current checkpoint: the Dioxus Android APK builds, installs and launches on a physical phone. Startup/native-library issues are fixed. The remaining work is manual end-to-end verification of picker/share/Termux/PC-discovery flows.

## Current verified state

| Item | Status |
|---|---|
| `dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose` | Passes |
| APK install on Samsung `SM_G781B` | Passes |
| Dioxus activity launch | Passes |
| First setup screen render | Passes |
| Crash buffer after smoke launch | Empty |
| Android launcher icon | Present through adaptive icon resources |

## Bridge module

Android bridge module path:

```text
android/bridge
```

Implemented adapters:

```text
DeepSeekDocumentPickerBridge.kt
DeepSeekPcGatewayDiscoveryBridge.kt
DeepSeekTermuxBridge.kt
DeepSeekMobileHostCoordinator.kt
NativeBridge.kt
TermuxResultReceiver.kt
```

The bridge manifest declares network/multicast permissions for PC discovery, Termux `RUN_COMMAND`, package visibility for Termux, and a file provider for native share/export flows.

## Dioxus packaging boundary

Dioxus generates a temporary Android project under `target/dx`. The project includes the Kotlin bridge module through:

```text
crates/mobile/src/android_plugin.rs
```

The metadata:

```rust
#[manganis::ffi("../../android/bridge")]
extern "Kotlin" {
    pub type DeepSeekMobileHostCoordinator;
}
```

This keeps the real bridge module in-repo while allowing `dx build --android` to package it into the generated Android host.

## Native library and JNI boundary

Dioxus packages the Rust Android native activity library as:

```text
libmain.so
```

Therefore `NativeBridge.kt` loads `main` first and falls back to `deepseek_mobile` only for standalone/reference shells.

Rust JNI exports must match the Kotlin class package:

```text
com.deepseek.mobile.bridge.NativeBridge
```

Current Rust export prefix:

```text
Java_com_deepseek_mobile_bridge_NativeBridge_*
```

## Custom Android manifest

Dioxus uses:

```toml
[application]
android_manifest = "../../android/AndroidManifest.xml"
```

The manifest is required because the default generated manifest did not handle all startup/config-change cases on the tested device. The activity keeps the full `configChanges` set, including `assetsPaths`, to avoid the native Dioxus activity restart crash seen during device testing.

The manifest also sets:

```xml
android:icon="@mipmap/deepseek_launcher"
android:roundIcon="@mipmap/deepseek_launcher_round"
```

## Rust command/callback boundary

Rust mobile bridge state lives in:

```text
crates/mobile/src/native_bridge.rs
crates/mobile/src/native_document_picker.rs
crates/mobile/src/native_pc_discovery.rs
crates/mobile/src/native_termux.rs
crates/mobile/src/native_event_router.rs
crates/mobile/src/android_host.rs
crates/mobile/src/native_host_runtime.rs
crates/mobile/src/jni_bridge.rs
```

The host shell drains pending native commands from `NativeBridgeState`, dispatches them to Kotlin adapters, then delivers JSON callbacks back into Rust.

## Document picker flow

The same picker command is used for chat attachments and project ZIP import. Rust distinguishes them through `DocumentPickerRequest.purpose`:

- `ChatAttachment` goes to the composer;
- `ProjectImport` imports the returned local archive copy into the phone workspace.

Flow:

1. Rust queues `NativeMobileCommand::OpenDocumentPicker`.
2. Host converts it with `pop_next_android_document_picker_command()`.
3. Android calls `DeepSeekDocumentPickerBridge`.
4. Android copies readable `content://` files into app-private sandbox storage.
5. Host forwards `AndroidDocumentPickerCallback` to Rust.
6. Rust rejects stale request ids and routes accepted documents by purpose.

Manual verification still required on hardware.

## PC discovery flow

1. Rust queues `NativeMobileCommand::StartPcGatewayDiscovery`.
2. Host converts it with `pop_next_android_pc_discovery_command()`.
3. Android NSD discovers `_deepseek-pc-host._tcp.` services.
4. Host forwards discovery callbacks to Rust.
5. Rust updates `PcPairingUiState`; an online route can become the active PC workspace.

Manual verification still required on a real LAN with `deepseek-pc-host` running.

## Termux command flow

The Settings UI persists and activates a Termux workspace connection from an absolute Termux path such as:

```text
/data/data/com.termux/files/home/project
```

The Termux bridge follows the Termux `RUN_COMMAND` intent contract:

- action: `com.termux.RUN_COMMAND`;
- service: `com.termux.app.RunCommandService`;
- command path: `/data/data/com.termux/files/usr/bin/sh`;
- arguments: `-lc`, followed by the approved command string;
- background execution: `true`;
- result transport: Android `PendingIntent` result bundle under the `result` extra.

Flow:

1. Core handles approved `exec_shell` on a Termux workspace and emits `termux_exec_request` metadata.
2. Rust mobile extracts that metadata and queues `NativeMobileCommand::RunTermuxCommand`.
3. Host converts it with `pop_next_android_termux_command()`.
4. Host creates a one-shot `PendingIntent` carrying the Rust `request_id`.
5. Android calls `DeepSeekTermuxBridge.run(...)`.
6. `TermuxResultReceiver` receives the result and delivers JSON back into Rust.
7. Rust rejects stale request ids and routes completion/failure into the mobile timeline.
8. `continue_termux_result` injects real command output back into the paused model turn.

Manual verification still required with real Termux permissions and `allow-external-apps=true`.

## Manual verification checklist before v1

- Pick one text/source file through Android picker and confirm it appears in the outgoing prompt.
- Import one project ZIP through Files → Import ZIP and confirm the local workspace refreshes.
- Export the phone workspace through Files → Export ZIP and confirm Android receives a native share command for the generated ZIP.
- Discover a running PC Host over mDNS and persist the active route.
- Save a valid Termux workspace path, run `pwd`, and confirm stdout, stderr, exit code, request id correlation and working directory.
- Send stale picker/discovery/Termux callbacks and confirm Rust rejects them.
- Restart the app and confirm persisted settings/workspace state load.

## Known-good smoke command

```powershell
. .\tools\android\env.ps1
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```
