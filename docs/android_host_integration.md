# Android host integration checklist

This document tracks the boundary between the Rust/Dioxus mobile code and the native Android bridge module.

The repository currently contains bridge contracts and Kotlin adapters, but not the final production Android host shell that drains Rust commands and forwards native callbacks. Until that host exists, this document is the manual integration contract.

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
```

The bridge manifest declares network/multicast permissions for PC discovery and `com.termux.permission.RUN_COMMAND` plus package visibility for Termux command execution.

## Rust command/callback boundary

Rust mobile bridge state lives in:

```text
crates/mobile/src/native_bridge.rs
crates/mobile/src/native_document_picker.rs
crates/mobile/src/native_pc_discovery.rs
crates/mobile/src/native_termux.rs
crates/mobile/src/native_event_router.rs
```

The host shell must repeatedly drain pending native commands from `NativeBridgeState` and dispatch them to Android. Native callbacks must be converted back into Rust callback enums and delivered to Rust state handling. Chat attachments and project import/export are routed in `main.rs` by `DocumentPickerRequest.purpose`; the standalone `route_native_mobile_event` helper remains useful for lower-level bridge tests and simple callback routing.

## Document picker flow

The same picker command is used for chat attachments and project ZIP import. Rust distinguishes them through `DocumentPickerRequest.purpose`: `ChatAttachment` goes to the composer, while `ProjectImport` imports the returned local archive copy into the phone workspace.

1. Rust queues `NativeMobileCommand::OpenDocumentPicker`.
2. Host converts it with `pop_next_android_document_picker_command()`.
3. Android calls `DeepSeekDocumentPickerBridge.buildIntent()` / launch flow.
4. Android copies readable `content://` files into app-private sandbox storage.
5. Host forwards `AndroidDocumentPickerCallback` to Rust.
6. Rust rejects stale request ids and routes accepted documents into `ChatComposerState`.

## PC discovery flow

1. Rust queues `NativeMobileCommand::StartPcGatewayDiscovery`.
2. Host converts it with `pop_next_android_pc_discovery_command()`.
3. Android NSD discovers `_deepseek-pc-host._tcp.` services.
4. Host forwards discovery callbacks to Rust.
5. Rust updates `PcPairingUiState`; an online route can become the active PC workspace.

## Termux command flow

The Rust/mobile settings UI now persists and activates a Termux workspace connection from an absolute Termux path such as `/data/data/com.termux/files/home/project`. The final Android host still needs device/emulator verification that this working directory reaches Termux correctly.

The Termux bridge follows the official Termux `RUN_COMMAND` intent contract:

- action: `com.termux.RUN_COMMAND`
- service: `com.termux.app.RunCommandService`
- command path: `/data/data/com.termux/files/usr/bin/sh`
- arguments: `-lc`, followed by the approved command string
- background execution: `true`, so stdout and stderr are returned separately
- result transport: Android `PendingIntent` result bundle under the `result` extra

Reference: https://github.com/termux/termux-app/wiki/RUN_COMMAND-Intent

Flow:

1. Core handles approved `exec_shell` on a Termux workspace in `ToolExecutionCoordinator` and emits tool-result metadata containing `termux_exec_request`.
2. Rust mobile extracts that metadata with `NativeBridgeState::enqueue_termux_command_from_agent_event()` and queues `NativeMobileCommand::RunTermuxCommand`.
3. Host converts it with `pop_next_android_termux_command()`.
4. Host creates a unique one-shot `PendingIntent` carrying the Rust `request_id`.
5. Android calls `DeepSeekTermuxBridge.run(command, pendingIntent)`.
6. Android parses the result bundle with `DeepSeekTermuxBridge.parseResult(requestId, resultIntent)`.
7. Host maps the payload to `AndroidTermuxCallback::Completed` or `AndroidTermuxCallback::Failed`.
8. Rust rejects stale request ids and routes completion/failure into the mobile timeline.
9. `NativeMobileEvent::TermuxCommandCompleted` triggers `continue_termux_result`, injecting the real command output back into the paused agent turn so the model can continue from the actual tool result.

Important host requirements:

- The app must request and the user must grant `com.termux.permission.RUN_COMMAND`.
- Termux must allow external apps in `~/.termux/termux.properties`.
- For Android package visibility, the host manifest must keep the Termux `<queries>` entry.
- PendingIntent request codes must be unique per command.
- Large stdout/stderr should be treated as bounded output; future executor plumbing should preserve truncation metadata when available.

## What is still not done

The repository now has the bridge contracts, but the final Android host still needs to close these pieces:

- Dioxus/native adapter that drains `NativeBridgeState` commands in the running Android app.
- Android service or receiver for Termux pending-intent results.
- Device/emulator verification that the host callback reaches the existing Rust/mobile `continue_termux_result` plumbing.
- Device/emulator verification of picker, PC discovery and Termux flows against the final host shell.

## Manual verification checklist

Before marking Android host integration complete:

- Pick one text/source file through Android picker; confirm it is copied into app-private storage and appears in the outgoing prompt.
- Import one project ZIP through Files → Import ZIP; confirm the archive local copy is extracted into the phone workspace and the Files view refreshes.
- Export the phone workspace through Files → Export ZIP; confirm Android receives a native share command for the generated `.zip` file.
- Discover a running PC host over mDNS; confirm the active route is visible and can be persisted as a workspace.
- Save a valid Termux workspace path in Settings, run a safe Termux command such as `pwd`, and confirm stdout, stderr, exit code, request id correlation and working directory are returned.
- Send a stale picker/discovery/Termux callback; confirm Rust rejects it and records an error instead of mutating active state.
- Restart the app; confirm persisted settings and active PC workspace still load.
