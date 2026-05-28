---
name: rust-android-dioxus-build-patterns
description: Practical Rust+Android+Dioxus build/run patterns for DeepSeek-Mobile (dx/cargo/adb).
---

## Purpose

Provide **repeatable commands** and guardrails for building and iterating on DeepSeek-Mobile (Rust + Dioxus) for Android from a Windows PC, plus “what to capture” when something breaks.

## Baseline environment (Windows)

From repo root, always load the Android toolchain environment first:

```powershell
. .\tools\android\env.ps1
```

If you need to refresh the toolchain setup on a new machine, use the repo scripts under `scripts/` (do not hand-wave installs).

## Build + install (fast path)

Build the debug APK and install it:

```powershell
. .\tools\android\env.ps1
dx build --android --package deepseek-mobile
.\scripts\adb-control.ps1 -Action InstallLaunch
```

Notes:

- `scripts/adb-control.ps1` expects the debug APK at:
  - `target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk`

## Build + install (explicit device)

If more than one device is connected:

```powershell
. .\tools\android\env.ps1
$serial = "<YOUR_SERIAL>"
dx build --android --package deepseek-mobile --device $serial
.\scripts\adb-control.ps1 -Action InstallLaunch -Serial $serial
```

## “What changed?” loop (safe iteration discipline)

Before changing build flags or “fixing” anything, capture:

```powershell
git status
git diff
git log -5 --oneline
```

Then:

- **Rebuild** (dx)
- **Install** (adb-control)
- **Capture evidence** (adb-control report + logcat)

```powershell
. .\tools\android\env.ps1
dx build --android --package deepseek-mobile
.\scripts\adb-control.ps1 -Action Report -ClearLogcat
.\scripts\adb-control.ps1 -Action Launch
.\scripts\adb-control.ps1 -Action Logcat
```

## Common build outputs to inspect (do not guess)

- APK exists:

```powershell
Test-Path ".\target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
```

- If missing, scroll back to the **first failing command** output. Do not treat later errors as root cause.

## Cargo-level debugging (when dx is not enough)

When you need Rust crate-level compilation detail, run cargo for the relevant crate (prefer staying within repo conventions). Capture the full output:

```powershell
cargo build -p deepseek-mobile 2>&1 | Tee-Object -FilePath .\target\cargo-deepseek-mobile-build.txt
```

If you change features or target triples, record them explicitly (in the artifact filename or a short summary file).

## Android runtime debugging checklist

When the app launches but behaves incorrectly:

- Get a structured report + screenshot + UI dump + logcat:

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action Report
```

- Look for:
  - `FATAL EXCEPTION`
  - `AndroidRuntime`
  - `ANR in com.deepseek.mobile`

## Dioxus UI patterns (operational constraints)

- Assume mobile UI must be responsive: avoid heavy work on the UI thread; defer IO and long tasks.
- Prefer incremental rendering for long lists/streams; avoid building huge DOM trees in one frame.
- When diagnosing UI: use `adb-control.ps1 -Action Tabs` to screenshot each tab and compare against expected navigation.

## “Stop-the-line” triggers (must triage before continuing)

- Build produces no APK
- `adb install` fails
- App shows a crash dialog or logcat has `FATAL EXCEPTION`
- Any ANR indicators in logcat or UI freeze > 5s during simple navigation

When triggered: collect evidence via `adb-control.ps1 -Action Full` and attach the generated `target/adb-control/...` directory.

