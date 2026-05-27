# ADB control center

**Updated:** 2026-05-27

Use this when a real Android phone is connected over USB debugging and you need repeatable diagnostics or direct control.

Main script:

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action Full -Serial <serial>
```

Artifacts are written to:

```text
target/adb-control/<timestamp>/
```

The script is intentionally safe by default: it does not clear app data unless `-Action ClearData` is explicitly requested.

## Common commands

```powershell
# Full smoke: launch app, collect report, open bottom-nav tabs, save screenshots/logcat/UI XML.
.\scripts\adb-control.ps1 -Action Full -Serial RFCNC0PWD4E -StayAwake -ClearLogcat

# Diagnostics only.
.\scripts\adb-control.ps1 -Action Report -Serial RFCNC0PWD4E

# Install latest debug APK and launch.
.\scripts\adb-control.ps1 -Action InstallLaunch -Serial RFCNC0PWD4E

# Safe Termux calibration request.
.\scripts\adb-control.ps1 -Action Calibrate -Serial RFCNC0PWD4E

# Open Termux and print the one-time setup commands instead of calibrating immediately.
.\scripts\adb-control.ps1 -Action Calibrate -Serial RFCNC0PWD4E -OpenTermux

# Manual UI control.
.\scripts\adb-control.ps1 -Action Tap -Serial RFCNC0PWD4E -X 70 -Y 145
.\scripts\adb-control.ps1 -Action Text -Serial RFCNC0PWD4E -Text "Reply with PONG"
.\scripts\adb-control.ps1 -Action Key -Serial RFCNC0PWD4E -KeyCode 4

# Raw adb shell passthrough.
.\scripts\adb-control.ps1 -Action Shell -Serial RFCNC0PWD4E -- dumpsys window
```

## What `Full` collects

- ADB device list and selected serial.
- Android model/version/SDK, screen size/density, battery.
- Focused window and app PID.
- Package dump and Termux package presence.
- App-private file listing under `files/deepseek-mobile`.
- Config snapshots: `config.json`, `termux_workspace.json`, `chat_sessions.json`, `workspace_connections.json`, `mcp.json`.
- Calibration trace and `.agent_calibrated_v1`, if present.
- Screenshots for launch and each bottom-nav tab.
- UIAutomator XML dumps.
- `logcat` excerpts with summary counters:
  - `fatal_hits`
  - `anr_hits`
  - `run_command_hits`

## Current verified device

Latest local run on 2026-05-27:

| Item | Result |
|---|---|
| Device | Samsung `SM_G781B`, Android 13 |
| Serial | `RFCNC0PWD4E` |
| Action | `Full` |
| Fatal exceptions | `0` |
| ANR hits | `0` |
| Termux RUN_COMMAND hits during passive UI smoke | `0` |

## Notes

- UIAutomator sees the Dioxus WebView as a coarse node; screenshots remain the primary visual evidence.
- The script can control taps/text/swipes, but coordinates are still device-resolution dependent.
- `Calibrate` requires Termux to have `allow-external-apps=true`; if not configured, run with `-OpenTermux` and follow the printed commands.
- Android system picker/share flows are disabled in the current preview build because they can ANR on the tested Samsung/Dioxus/Wry stack. Use PC Host, ADB push, or the app workspace until that lifecycle issue is fixed.
