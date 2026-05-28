# Device setup (Android)

**Updated:** 2026-05-29

## 0. Install or update the APK (one command)

The app binary is **not** in git — build and install from your PC checkout. Chats, settings, and workspace on the phone are preserved on upgrade.

```powershell
cd $HOME\DeepSeek-Mobile   # or your clone path
. .\tools\android\env.ps1
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Launch
```

After `git pull`, run the same command (or add `-Pull` to update source and APK together). See [`INSTALL_UPDATE.md`](./INSTALL_UPDATE.md).

First-time alias: `.\scripts\install-phone-apk.ps1 -Serial RFCNC0PWD4E` (same as above with `-Launch`).

APK path: `target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk`

**No in-app auto-update** — use the script whenever the repo changes.

## 1. API key (project `.env`)

Debug builds can prefill onboarding from repo `.env` (`DEEPSEEK_API_KEY=sk-…`). Release builds do not embed `.env`.

## 2. Termux (required for TUI-class agent)

**Product priority:** full phone agent first; PC Host pairing is a later phase.

1. Install **Termux** from F-Droid (or use **Установить Termux** on the in-app setup screen).
2. In Termux (one-time, cannot be set from our app):
   ```bash
   mkdir -p ~/.termux
   echo allow-external-apps=true >> ~/.termux/termux.properties
   termux-reload-settings
   ```
3. Grant **RUN_COMMAND** when Termux prompts (or tap **Проверить RUN_COMMAND** on setup).
4. Set project path in setup, e.g. `/data/data/com.termux/files/home/deepseek-project`.
5. Continue — app seeds workspace via background calibration when bridge is ready.

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
