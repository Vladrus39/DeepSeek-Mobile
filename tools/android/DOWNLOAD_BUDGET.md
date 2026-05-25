# Download budget to finish DeepSeek-Mobile (Android + release)

All figures are for **Windows**, **first-time** installs. If you already have caches elsewhere, real traffic can be lower.

## Already satisfied (no download)

| Item | Status |
|------|--------|
| Rust / Cargo | Installed |
| JDK 21 | Installed |
| SDK slice (platform-tools, build-tools 35, platform 36) | Copied into `tools/android/sdk/` (~255 MB on disk, **0 MB** download) |
| Project code, tests | In repo |
| Android Studio on `D:\Project V` | Optional; **not required** if using `dx` + local SDK |

## Minimum to build Android APK (`dx build android`)

| # | Component | Size (download) | Notes |
|---|-----------|---------------|--------|
| 1 | **Android NDK** 26.1.10909125 | **~631 MB** | Zip; unpack to `tools/android/sdk/ndk/26.1.10909125/` |
| 2 | **dioxus-cli** (`dx`) | **~250–450 MB** | `cargo install dioxus-cli --locked` (crates.io + compile; cache may shrink retries) |
| 3 | **Rust Android targets** | **~90–130 MB** | `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android` |
| | **Minimum total** | **~970 MB – 1.2 GB** | One-time |

## Recommended (comfortable dev, not strict minimum)

| # | Component | Size (download) | Notes |
|---|-----------|---------------|--------|
| 4 | Platform **android-35** (optional) | **~100 MB** | Project `compileSdk = 35`; android-36 often works; add if Gradle complains |
| 5 | **cmdline-tools** (sdkmanager) | **~150 MB** | Only if you want CLI NDK install instead of manual zip |
| 6 | Emulator system image (e.g. API 35) | **~0.8–1.5 GB** | Only for emulator; **physical phone = skip** |
| | **With emulator** | **+1.0–1.7 GB** | |

## Do not download for this project

| Item | Why skip |
|------|----------|
| Full SDK again (~4 GB) | Already copied minimal slice + system SDK on C: |
| `D:\Project V\Android Studio` copy | 3.1 GB; use existing install or IDE you already have |
| `Help Car.rar` (5 GB) | Unrelated archive |

## Scenarios

| Goal | Budget |
|------|--------|
| **Code + tests only** | **0 MB** (done today: `cargo test --workspace`) |
| **APK on a real phone** | **~1.0–1.2 GB** (NDK + dx + Rust targets) |
| **APK + emulator** | **~2.0–2.9 GB** |
| **Signed store release** | Same as APK + keystore (negligible) |

## After internet is available (ordered)

```powershell
# 1) NDK — manual zip into tools/android/sdk/ndk/26.1.10909125/

# 2) Rust targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

# 3) Dioxus CLI (match workspace Dioxus 0.7)
cargo install dioxus-cli --version "0.7.9" --locked

# 4) Build
. .\tools\android\env.ps1
cd C:\Users\vladi\Desktop\DeepSeek-Mobile
dx build android
```

Update the `dioxus-cli` version if `Cargo.toml` workspace `dioxus` version changes.
