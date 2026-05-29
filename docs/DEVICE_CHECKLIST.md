# Device verification checklist

**Device:** `RFCNC0PWD4E`  
**Date:** 2026-05-30  
**App version on device:** 0.1.1 (debug)

Automated probes run from repo root with USB debugging enabled.

| Check | Script | Result |
|-------|--------|--------|
| ZIP import (headless) | `device-e2e-zip-import.ps1` | **PASS** |
| ZIP export + share | `device-e2e-zip-export.ps1` | **PASS** |
| Termux `pwd` + continuation | `device-termux-pwd-probe.ps1` | **PASS** |
| PC Host discovery / health | `device-e2e-pc-host.ps1 -SkipBuild` | **PASS** (manual URL; mDNS blocked on LAN) |
| PC pairing ZIP export + PC launch | `device-e2e-pc-pairing-bundle.ps1` | **PASS** (export + `Setup-DeepSeek-PC-Host`; phone→PC health may need firewall on first run) |
| API probe | `device-full-verify.ps1 -SkipBuild` | **PASS** |
| Agent turn probe | `device-full-verify.ps1 -SkipBuild` | **PASS** (`PROBE_OK`) |
| Termux calibration file | `device-full-verify.ps1 -SkipBuild` | **PASS** |
| PC mDNS discovery probe | `device-full-verify.ps1 -SkipBuild` | **PASS** (manual URL `http://192.168.1.111:8787`) |
| Full verify bundle | `run-device-checklist.ps1 -SkipBuild` | **PASS** except pairing-bundle phone→PC health (firewall) |

## GitHub Release / OTA

- **Release:** [v0.1.1](https://github.com/Vladrus39/DeepSeek-Mobile/releases/tag/v0.1.1) — asset `deepseek-mobile-0.1.1.apk`
- **In-app:** Settings → App update (works when installed version &lt; 0.1.1)
- **Dev signing:** release APK signed with standard Android debug keystore so it can upgrade debug installs

## Chat → Files

- **Work log:** «Открыть папку проекта» opens the Files tab at workspace root; per-file links only inside expanded tool steps.
- **Rollback:** «Откатить к safety snapshot» in work log with inline confirm in chat.

## Still manual

- Document picker with system UI (chat attachment) — no headless probe yet
- Full UI walkthrough / touch ergonomics on all cockpit panels
- Play Store AAB submission

## PC-side note

Pairing ZIP setup (`Setup-DeepSeek-PC-Host.cmd`) adds firewall rules automatically (one-time UAC). For manual `cargo run -p deepseek-pc-host` from a dev checkout, allow **TCP 8787** (and UDP 5353 for mDNS):

```powershell
# From repo root (or use full path to the .ps1 — not from C:\Windows\System32):
cd C:\Users\vladi\Desktop\DeepSeek-Mobile
.\scripts\enable-pc-host-mdns-windows.ps1
# Or: right-click enable-pc-host-firewall.cmd -> Run as administrator
```
