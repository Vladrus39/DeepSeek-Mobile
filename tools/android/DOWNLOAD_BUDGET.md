# Download budget to finish DeepSeek-Mobile Android work

**Updated:** 2026-05-26

The debug APK path is now working on the connected physical phone. No additional download is currently required for the existing smoke-test flow.

## Already satisfied on this machine

| Item | Status |
|---|---|
| Rust / Cargo | Installed |
| JDK | Installed |
| Project-local Android SDK slice | Present under `tools/android/sdk/` |
| Android NDK 26.1.10909125 | Present under `tools/android/sdk/ndk/26.1.10909125/` |
| Rust Android targets | Installed |
| Dioxus CLI | `dx 0.7.9` installed |
| Gradle wrapper/cache needed for debug build | Present |
| USB device debug path | Verified with Samsung `SM_G781B` / `RFCNC0PWD4E` |

## Current minimum build command

```powershell
. .\tools\android\env.ps1
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```

Expected output APK:

```text
target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
```

## Remaining downloads

| Goal | Extra download estimate | Required now? |
|---|---:|---|
| Continue debug APK testing on the connected phone | 0 MB | No |
| Android emulator image | ~0.8–1.5 GB | Optional; skip while using physical phone |
| Extra SDK platforms/build tools | ~100–300 MB | Only if Gradle asks for a missing version |
| Release signing tools | 0 MB | No meaningful download; keystore must stay outside repo |
| PC Host release packaging deps | Depends on target platform | Later |

## Do not download for this project

| Item | Why skip |
|---|---|
| Full Android SDK again | Repo-local SDK/NDK path already works |
| `D:\Project V` Android Studio copy | Not required by this repository |
| Large unrelated archives | Not part of the build |

## If the build cache is deleted

Likely one-time re-downloads:

| Component | Approximate size |
|---|---:|
| Android NDK 26.1.10909125 | ~631 MB |
| Dioxus CLI crates/compile cache | ~250–450 MB |
| Rust Android targets | ~90–130 MB |
| Gradle wrapper distribution | ~128 MB |

With the current local state, these are already available.
