# Android document picker bridge

This document defines the native Android side of the DeepSeek Mobile document picker contract.

See also: `docs/android_host_integration.md` for the wider host shell checklist covering picker, PC discovery, and Termux.

The Rust mobile layer exposes a command/callback model:

- command: `AndroidDocumentPickerCommand`
- callback: `AndroidDocumentPickerCallback`
- result payload: `AndroidPickedDocumentPayload`
- bridge state: `NativeBridgeState::active_document_picker_request_id`

The repository also contains a production Android bridge module at:

```text
android/bridge
```

The module implements `DeepSeekDocumentPickerBridge`, which opens the system picker, copies readable `content://` documents into app-private sandbox storage, and returns a payload with `localPath` for Rust-side attachment ingestion.

## Command consumed by Android

When Rust drains a `NativeMobileCommand::OpenDocumentPicker`, it is converted into:

```text
AndroidDocumentPickerCommand {
    request_id: String,
    action: "android.intent.action.OPEN_DOCUMENT",
    category: "android.intent.category.OPENABLE",
    allow_multiple: bool,
    mime_types: Vec<String>,
}
```

Android must keep `request_id` and return it unchanged in the callback. Rust rejects stale callbacks where `request_id` does not match `active_document_picker_request_id`.

## Android module

The Android bridge module is intentionally separate from the Rust workspace so it can be attached to the final Dioxus Android host app without changing the Rust crate graph.

Files:

```text
android/bridge/build.gradle.kts
android/bridge/src/main/AndroidManifest.xml
android/bridge/src/main/kotlin/com/deepseek/mobile/bridge/DeepSeekDocumentPickerBridge.kt
android/bridge/consumer-rules.pro
```

The host Android app should include this module and instantiate `DeepSeekDocumentPickerBridge` from its `ComponentActivity`.

## Android intent behavior

The native layer creates an `Intent` equivalent to:

```kotlin
Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
    addCategory(Intent.CATEGORY_OPENABLE)
    putExtra(Intent.EXTRA_ALLOW_MULTIPLE, command.allowMultiple)
    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
    type = if (command.mimeTypes.size == 1) command.mimeTypes[0] else "*/*"
    putExtra(Intent.EXTRA_MIME_TYPES, command.mimeTypes.toTypedArray())
}
```

No broad storage permission is required for `ACTION_OPEN_DOCUMENT`.

## Required sandbox copy

For every selected `Uri`, Android copies the stream into app-private storage before returning it to Rust.

Location:

```text
<context.filesDir>/attachments/<request_id>/<safe_display_name>
```

The Rust ingestion layer reads text/source attachments from `local_path`. A raw `content://` URI without `local_path` is not enough for prompt ingestion, because Rust cannot reliably access Android `ContentResolver` streams directly.

## Callback payload

For every selected file, Android returns:

```text
AndroidPickedDocumentPayload {
    id: String,
    display_name: String,
    uri: Option<String>,
    local_path: Option<String>,
    mime_type: Option<String>,
    size_bytes: Option<u64>,
}
```

`local_path` must point to the sandbox copy when copying succeeds.

## Callback outcomes

Successful pick:

```text
AndroidDocumentPickerCallback::Picked {
    request_id,
    documents,
}
```

User cancelled:

```text
AndroidDocumentPickerCallback::Cancelled {
    request_id,
}
```

Native failure:

```text
AndroidDocumentPickerCallback::Failed {
    request_id,
    message,
}
```

## Integration sketch

The exact transport back into Rust depends on the final Dioxus mobile/native binding used by the Android shell. The host must map Kotlin payloads into the Rust callback shape:

```text
AndroidDocumentPickerCallback::Picked { request_id, documents }
AndroidDocumentPickerCallback::Cancelled { request_id }
AndroidDocumentPickerCallback::Failed { request_id, message }
```

Then pass the callback into:

```text
NativeBridgeState::accept_android_document_picker_callback(callback)
route_native_mobile_event(..., returned_event)
```

After routing, selected documents are attached to `ChatComposerState`; on Send, `to_core_input_with_ingestion()` reads local text/source files and injects extracted text into the prompt.
