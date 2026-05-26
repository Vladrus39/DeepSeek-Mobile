# DeepSeek-Mobile — isolated Android toolchain

Everything under `tools/android/` belongs only to this repository. It does not use `D:\Project V` or another project SDK.

## Current status — 2026-05-26

| Component | Path | Status |
|---|---|---|
| platform-tools / adb | `sdk/platform-tools/` | Present |
| Android platform | `sdk/platforms/android-36/` | Present |
| Android build tools | `sdk/build-tools/` | Present |
| Android NDK | `sdk/ndk/26.1.10909125/` | Present |
| SDK license cache | `sdk/licenses/` | Local only, git-ignored |
| Dioxus CLI | user cargo bin, `dx 0.7.9` | Present |
| Rust Android targets | rustup | Present |

## Activate for a terminal session

```powershell
. .\tools\android\env.ps1
adb devices
```

## Known-good debug build

```powershell
. .\tools\android\env.ps1
dx build --android --package deepseek-mobile --device RFCNC0PWD4E --verbose
```

The latest successful smoke test installed and launched the APK on Samsung `SM_G781B` / serial `RFCNC0PWD4E`.

## Gradle / reference Android app

`android/app/local.properties` points at this SDK. The Dioxus-generated project also uses this environment when launched after sourcing `env.ps1`.

## Repository hygiene

The large SDK/NDK folders, downloads, licenses and SDK cache files are ignored by git. Do not commit local Android SDK contents.

## Download notes

See [DOWNLOAD_BUDGET.md](./DOWNLOAD_BUDGET.md) for what is already present and what may still be optional for emulator/release workflows.
