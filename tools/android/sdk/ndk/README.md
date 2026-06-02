# NDK (not copied — install when online)

Rust/Dioxus Android builds need the side-by-side NDK. It is **not** copied into git (too large).

## Install into this project SDK (recommended)

1. Download **NDK 26.1.10909125** for Windows (~**631 MB** zip).
2. Unzip so this path exists:

```text
tools/android/sdk/ndk/26.1.10909125/
```

3. Run `. .\tools\android\env.ps1` — `ANDROID_NDK_HOME` will be set automatically.

## Or install via Android Studio

SDK Manager → NDK (Side by side) **26.1.10909125** → same folder layout under `tools/android/sdk/ndk/`.
