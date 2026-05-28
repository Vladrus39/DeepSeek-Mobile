---
name: adb-e2e-testing-discipline
description: Reproducible Android E2E via ADB: logcat, screenshots, UI dumps, run-as files, scripts.
---

## Purpose

Make Android E2E runs **reproducible** and **evidence-driven**. Every claim about behavior must be backed by artifacts: screenshots, UI XML, `logcat`, and app files captured via `run-as`.

## Preferred tooling (this repo)

Use these scripts first:

- `scripts/adb-control.ps1`: report/capture/logcat/tabs/install/launch + safe evidence collection
- `scripts/device-e2e-verify.ps1`: PC Host health + device build/install + Termux calibration checks

## Minimal evidence for any UI/behavior bug

Collect in one shot:

```powershell
. .\tools\android\env.ps1
.\scripts\adb-control.ps1 -Action Report
```

This captures:

- Screenshot (`*-screen.png`)
- UI dump (`*-ui.xml`)
- `logcat` tail (`*-logcat.txt`)
- Selected internal app files via `run-as` when possible

If reproducing a multi-step UI sequence, run:

```powershell
.\scripts\adb-control.ps1 -Action Full -ClearLogcat
```

## Logcat discipline

Rules:

- Clear logcat **only when you’re about to reproduce** and you will capture immediately after.
- Always capture enough lines to include the crash root and surrounding context.

Capture after reproducing:

```powershell
.\scripts\adb-control.ps1 -Action Logcat
```

What to search for (in captured logcat file):

- `FATAL EXCEPTION`
- `AndroidRuntime`
- `ANR in com.deepseek.mobile`
- `Input dispatching timed out`
- `DeepSeekTermux`, `RUN_COMMAND`

## Screenshot + UI XML discipline

Use `Report` for a snapshot; use `Tabs` for navigation sanity:

```powershell
.\scripts\adb-control.ps1 -Action Tabs
```

If you need a single shot at a specific moment:

```powershell
.\scripts\adb-control.ps1 -Action Capture
```

## run-as / internal files discipline

Internal evidence lives under:

- `files/deepseek-mobile/` (inside app sandbox)

If `run-as` fails (“not debuggable”), do not pretend you can read internal files. Switch to:

- external files dir for pushing skills/config
- logcat + screenshots for evidence

## Chat/turn verification (ground truth)

If you are validating assistant behavior, capture persisted session state (if available):

```powershell
.\scripts\adb-control.ps1 -Action ChatSend -Text "Ping" -ClearLogcat
```

Then inspect the generated output directory (it captures `chat_sessions.json` and runtime session files when accessible).

## When to stop and escalate (must not continue “blind”)

- Any crash dialog or `FATAL EXCEPTION`
- Any ANR hit
- Termux calibration missing after a reasonable wait
- UI tab navigation inconsistent or missing elements across `Tabs` screenshots

Escalation = run `-Action Full`, attach `target/adb-control/<timestamp>/` as the canonical reproduction bundle.

