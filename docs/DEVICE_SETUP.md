# Device setup (Android)

**Updated:** 2026-05-29

## 0. Install or update the APK (one command)

For day-to-day development, the app binary is **not** stored in git — build and install it from your PC checkout. Chats, settings, and workspace on the phone are preserved on upgrade because the scripts use `adb install -r`.

```powershell
cd $HOME\DeepSeek-Mobile   # or your clone path
. .\tools\android\env.ps1
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Launch
```

After `git pull`, run the same command again, or add `-Pull` to update source and APK together. See [`INSTALL_UPDATE.md`](./INSTALL_UPDATE.md).

```powershell
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Pull -Launch
```

First-time alias: `./scripts/install-phone-apk.ps1 -Serial RFCNC0PWD4E` (same install/launch path).

APK path:

```text
target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
```

**Developer default:** use `update-phone-apk.ps1` after source changes.

**Release/OTA path:** optional GitHub Releases + in-app updater exists, but it depends on publishing a signed release asset named `deepseek-mobile-<version>.apk`. Until you publish releases, the reliable update path is USB/ADB from the PC.

## 1. API key (project `.env`)

Debug builds can prefill onboarding from repo `.env` (`DEEPSEEK_API_KEY=sk-…`). Release builds do not embed `.env`.

## 2. Termux (required for TUI-class agent) — now much easier

**Product priority:** full phone agent first; PC Host pairing is optional for large repos / desktop toolchains.

In the app's first-run **Setup** screen there is now a guided flow with big buttons:

1. Tap **Install Termux** or **Open Termux** (opens F-Droid or launches the app).
2. Tap the big green **"Grant permission & auto-setup Termux"** button.
   - This sends a safe command that triggers Termux's RUN_COMMAND permission dialog.
   - Once you grant it in Termux, the app automatically queues:
     - Configuration of `allow-external-apps=true`
     - Creation and seeding of the default workspace (`/data/data/com.termux/files/home/deepseek-project`)
3. The path is auto-filled. Review API key, tap **Continue**.

You may still need to restart Termux once after the properties are written (Termux shows a toast).

The old manual steps (editing properties by hand, mkdir) are still possible as fallback using the smaller "Auto-config properties" / "Seed workspace" buttons.

After Continue the app will run background calibration/probes and you should see "Termux OK" and be able to use the full agent.

Offline USB install: `scripts/install-termux-offline.ps1`.

## 3. Smoke tests on device

**Do not** run `device-full-verify.ps1` or other probe scripts during manual chat testing — they `force-stop` the app.

Scripted E2E only:

```powershell
. .\tools\android\env.ps1
.\scripts\device-termux-pwd-probe.ps1 -Serial RFCNC0PWD4E
```

## 4. Data locations (Android)

| Path | Contents |
|------|----------|
| `<filesDir>/deepseek-mobile/` | config, secrets, runtime_store, workspace |
| `<filesDir>/deepseek-mobile/workspace/` | app sandbox project |
| Termux path you configured | full shell/git/cargo |

## 5. Helper scripts

```powershell
. .\tools\android\env.ps1
.\scripts\device-provision.ps1 -Serial RFCNC0PWD4E
.\scripts\device-smoke.ps1 -Serial RFCNC0PWD4E
```

`device-provision.ps1` checks app-private storage and Termux presence. If Termux is missing, it opens the F-Droid Termux page on the device.

## 6. ADB helpers

**Canonical control script:** [`scripts/adb-control.ps1`](../scripts/adb-control.ps1) — install/launch, capture, Termux grant, chat send. See [`docs/ADB_CONTROL.md`](./ADB_CONTROL.md).

Preferred one-command APK update: [`scripts/update-phone-apk.ps1`](../scripts/update-phone-apk.ps1).

```powershell
. .\tools\android\env.ps1
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -SkipBuild -Launch
# or install/launch only if APK already built:
.\scripts\adb-control.ps1 -Action InstallLaunch -Serial RFCNC0PWD4E
adb devices
adb logcat -s "DeepSeek" "RustStdout" "dioxus" | Select-Object -Last 80
adb shell run-as com.deepseek.mobile ls files/deepseek-mobile
```

## 7. Troubleshooting

- **Onboarding every launch** — data dir not initialized; reinstall latest APK (includes `NativeBridge.initMobileDataDir`).
- **API errors** — check Health → API configured; re-save key in Settings.
- **Termux silent** — `allow-external-apps`, RUN_COMMAND permission, absolute path.
- **Plan mode** — switch to **Agent** in Settings (tools disabled in Plan).
- **Work log stuck on «выполняется»** — fixed in 2026-05-28 build; update APK.
- **EOF restore banner** — fixed; corrupt event JSON is skipped.

See also [`TROUBLESHOOTING.md`](./TROUBLESHOOTING.md).
