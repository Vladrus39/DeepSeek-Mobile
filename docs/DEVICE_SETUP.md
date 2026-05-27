# Real device setup — full agent on phone

Use this after `dx build --android` installs the debug APK (see `docs/CURRENT_STATE.md`).

## 1. API key (project `.env`)

1. Copy `.env.example` → `.env` at repo root.
2. Set `DEEPSEEK_API_KEY=sk-...` (gitignored).
3. Rebuild debug APK: `dx build --android --package deepseek-mobile --device <serial>`.

On **desktop** debug builds, an empty secrets store may be auto-filled from `.env` for faster iteration.

On **Android**, the debug APK may **prefill** the onboarding field from your machine’s `.env` at build time, but the key is saved only after you tap **Continue** / save in Settings.

**Release builds do not embed `.env`.** Everyone enters their own key on the device. Do not share debug APKs built with a real `.env` key.

## 2. Termux (required for TUI-class agent)

### Phone has no internet

On your PC (with internet):

```powershell
. .\tools\android\env.ps1
.\scripts\install-termux-offline.ps1 -Serial RFCNC0PWD4E
```

This downloads the F-Droid Termux APK (~109 MB) and a bootstrap zip fallback, installs via USB, and copies `bootstrap-aarch64.zip` to `Download/` on the phone. Termux 0.118+ also embeds bootstrap in the APK, so first launch usually works offline after 1–2 minutes.

Then on the phone in Termux (no network):

```bash
mkdir -p ~/deepseek-project
mkdir -p ~/.termux
echo allow-external-apps=true >> ~/.termux/termux.properties
```

Restart Termux, open DeepSeek Mobile, grant **Run commands in Termux environment**.

### Phone has internet

1. Install [Termux](https://github.com/termux/termux-app) from F-Droid.
2. In Termux:
   ```bash
   mkdir -p ~/deepseek-project && cd ~/deepseek-project
   echo "allow-external-apps=true" >> ~/.termux/termux.properties
   ```
   Restart Termux after editing `termux.properties`.
3. Grant **Run commands in Termux environment** when Android prompts (or Termux:API permission flow).
4. In DeepSeek Mobile → onboarding or **Settings → Termux workspace**, set path:
   ```text
   /data/data/com.termux/files/home/deepseek-project
   ```
5. Save and open **Health** — expect “full agent on phone ready” when API + path are valid.

## 3. Smoke tests on device

| Test | Where | Expected |
|------|--------|----------|
| Chat | Agent mode, send “list files in workspace” | Model replies; may call `list_dir` |
| Shell | “run pwd in termux” / quick action | Termux runs; timeline shows output; model continues |
| Files | Import ZIP | Picker → workspace updates |
| Approvals | `write_file` with review mode | Approve → tool runs → **new assistant message** |
| PC (optional) | PC Host panel | mDNS or manual URL when host running on LAN |

## 4. Data locations (Android)

| Path | Content |
|------|---------|
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

```powershell
adb devices
adb logcat -s "DeepSeek" "RustStdout" "dioxus" | Select-Object -Last 80
adb shell run-as com.deepseek.mobile ls files/deepseek-mobile
```

## 7. Troubleshooting

- **Onboarding every launch** — data dir not initialized; reinstall latest APK (includes `NativeBridge.initMobileDataDir`).
- **API errors** — check Health → API configured; re-save key in Settings.
- **Termux silent** — `allow-external-apps`, RUN_COMMAND permission, absolute path.
- **Plan mode** — switch to **Agent** in Settings (tools disabled in Plan).
