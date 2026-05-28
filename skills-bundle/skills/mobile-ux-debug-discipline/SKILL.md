---
name: mobile-ux-debug-discipline
description: Avoid ANR on Android; document picker/share on UI thread; chat scroll and inline approvals.
---

## Purpose

Keep DeepSeek-Mobile responsive on Android while using native flows (picker, share, Termux).

## ANR prevention

- Launch **document picker**, **share sheet**, and **external URLs** only from the **main/UI thread**.
- Do not block the Dioxus render loop on file I/O or MCP HTTP; use background tasks + callbacks.
- After starting an external Activity, expect the app to pause — persist state before opening picker/share.

## ZIP import/export

- **Import ZIP**: `OPEN_DOCUMENT` + `application/zip`; copies URI into app sandbox before unzip.
- **Export ZIP**: writes under `files/deepseek-mobile/exports/` then shares via `FileProvider` (`<package>.fileprovider`).
- If share fails with `FileUriExposedException` or `file not found`, verify export path is under `filesDir` and `file_paths.xml` includes `files-path`.

## Package visibility (Android 11+)

Manifest must declare `<queries>` for `OPEN_DOCUMENT`, `SEND`, and `CHOOSER` so pickers/targets resolve.

## Chat UX

- Conversation list uses **newest-at-bottom** scroll; after restore, scroll to last message.
- **Approvals** render inline on the timeline item that requested them (not a separate blocking panel).

## Debugging

```powershell
adb logcat -c
# reproduce Import/Export
adb logcat -d | Select-String -Pattern "deepseek|DocumentPicker|share|FileProvider|AndroidRuntime"
```

## Headless probes (ADB)

- `.zip_transfer_probe_requested` → export + share result in `.zip_transfer_probe_result`
- `.agent_turn_probe_requested` → agent turn smoke

Always read probe result files under `run-as com.deepseek.mobile` → `files/deepseek-mobile/`.
