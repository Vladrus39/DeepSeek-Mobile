# DeepSeek-Mobile — isolated Android toolchain

Everything under `tools/android/` belongs **only** to this repo. It does not use `D:\Project V` or other projects.

## Already on disk (copied locally, no internet)

| Component | Path | Size |
|-----------|------|------|
| platform-tools (adb, etc.) | `sdk/platform-tools/` | ~16 MB |
| build-tools 35.0.0 | `sdk/build-tools/35.0.0/` | ~138 MB |
| platform android-36 | `sdk/platforms/android-36/` | ~101 MB |
| **Total local SDK slice** | `sdk/` | **~255 MB** |

Refresh from system SDK (still no internet):

```powershell
.\tools\android\sync-sdk-from-system.ps1
```

## Activate for a terminal session

```powershell
. .\tools\android\env.ps1
adb devices
```

## Gradle / reference Android app

`android/app/local.properties` points at this SDK.

## See download budget

Exact sizes for what is still missing: **[DOWNLOAD_BUDGET.md](./DOWNLOAD_BUDGET.md)**.
