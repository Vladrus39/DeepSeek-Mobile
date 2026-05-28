---
name: termux-first-execution-discipline
description: Tool-verified Termux-first execution on Android; never claim results without logs.
---

## Purpose

Enforce a **Termux-first** discipline when working with DeepSeek-Mobile on-device execution, especially around the Termux bridge (`RUN_COMMAND`). Prevent “imagined success” by requiring **observable evidence** (tool output, captured logs, files).

## Non‑negotiables (must follow)

- **Never claim a command ran** unless you have **captured output** from:
  - `adb shell ...` / `adb logcat ...`, or
  - an app-produced file in `files/deepseek-mobile/`, or
  - a PC-host script output.
- **Prefer device-local execution** (Termux / on-device file ops) over describing what *should* happen.
- **When Termux is pending / unconfigured**, stop and drive setup steps first; do not proceed with later steps.
- **Record evidence per step**: command + output + timestamped artifact path.

## Required evidence bundle per attempt

For every workflow where you expect Termux to execute:

- **ADB report directory** created by `scripts/adb-control.ps1`, OR a manual `target/` folder you create.
- At least one of:
  - `logcat` excerpt containing `RUN_COMMAND`, `DeepSeekTermux`, or a clear failure marker
  - `files/deepseek-mobile/.calibration_trace` content
  - `files/deepseek-mobile/.agent_calibrated_v1` equals `ok`

## Termux bridge prerequisites checklist

On PC (repo root):

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action Report
```

Verify:

- Device connected (script selects serial or fails with a captured `adb-devices.txt`)
- App package is `com.deepseek.mobile`
- `run-as` is **OK** in the report summary (otherwise internal files cannot be inspected/copied)

Grant permission (idempotent):

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action GrantTermux
```

On the phone (once):

- Open **Termux**
- Ensure it finishes initial bootstrap (package updates, storage prompt if shown)
- Run:

```sh
mkdir -p ~/.termux
echo allow-external-apps=true >> ~/.termux/termux.properties
termux-reload-settings
```

## Calibration: the fastest ground-truth test

Run the end-to-end calibration check:

```powershell
. .\tools\android\env.ps1
.\scripts\device-e2e-verify.ps1
```

Interpretation (do not guess):

- **PASS**: `termux_calibration = PASS` and `calibration_trace` shows `marked_ok`
- **FAIL**: capture `target/...` output + the report’s `...-logcat.txt` and `calibration-trace.txt`

If calibration doesn’t complete:

- Open Termux and re-run the “once” steps above
- Re-run `device-e2e-verify.ps1`
- If still failing: run `.\scripts\adb-control.ps1 -Action Full -OpenTermux` and attach the generated artifacts

## When something goes wrong (triage protocol)

1) **Stop** and collect evidence:

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action Report -ClearLogcat
.\scripts\adb-control.ps1 -Action Logcat
```

2) Check for common failure modes:

- **Permission not granted**: `dumpsys package com.deepseek.mobile` missing `RUN_COMMAND: granted=true`
- **Termux missing**: `pm list packages com.termux` empty
- **Termux disallows external apps**: no `allow-external-apps=true`
- **run-as blocked**: “not debuggable” (you can still use external files dir, but internal inspection is limited)

3) Only after collecting evidence: propose fixes tied to the observed failure mode.

