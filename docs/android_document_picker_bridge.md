# Android document picker bridge

This document defines the native Android side of the DeepSeek Mobile document picker contract.

The Rust mobile layer already exposes a command/callback model:

- command: `AndroidDocumentPickerCommand`
- callback: `AndroidDocumentPickerCallback`
- result payload: `AndroidPickedDocumentPayload`

The native Android layer must open the system picker, copy readable `content://` documents into the app sandbox, and send the callback back to Rust/Dioxus.

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

Android must keep `request_id` and return it unchanged in the callback.

## Android intent behavior

The native layer should create an `Intent` equivalent to:

```kotlin
Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
    addCategory(Intent.CATEGORY_OPENABLE)
    putExtra(Intent.EXTRA_ALLOW_MULTIPLE, command.allowMultiple)
    type = if (command.mimeTypes.size == 1) command.mimeTypes[0] else "*/*"
    putExtra(Intent.EXTRA_MIME_TYPES, command.mimeTypes.toTypedArray())
}
```

## Required sandbox copy

For every selected `Uri`, Android should copy the stream into app-private storage before returning it to Rust.

Recommended location:

```text
<context.filesDir>/attachments/<request_id>/<safe_display_name>
```

The Rust ingestion layer reads text/source attachments from `local_path`. A raw `content://` URI without `local_path` is not enough for prompt ingestion, because Rust cannot reliably access Android `ContentResolver` streams directly.

## Callback payload

For every selected file, Android should return:

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

The Rust bridge rejects stale callbacks where `request_id` does not match the active picker request.

## Minimal Kotlin reference logic

```kotlin
private fun copyUriToSandbox(requestId: String, uri: Uri): PickedPayload {
    val resolver = applicationContext.contentResolver
    val mime = resolver.getType(uri)
    val displayName = queryDisplayName(uri) ?: uri.lastPathSegment ?: "attachment"
    val safeName = displayName.replace(Regex("[^A-Za-z0-9._-]"), "_")
    val dir = File(applicationContext.filesDir, "attachments/$requestId")
    dir.mkdirs()
    val outFile = File(dir, safeName)

    resolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "Cannot open input stream for $uri" }
        outFile.outputStream().use { output -> input.copyTo(output) }
    }

    return PickedPayload(
        id = UUID.randomUUID().toString(),
        displayName = displayName,
        uri = uri.toString(),
        localPath = outFile.absolutePath,
        mimeType = mime,
        sizeBytes = outFile.length(),
    )
}
```

The exact transport back into Rust depends on the final Dioxus mobile/native binding used by the Android shell, but the callback data must preserve this contract.
