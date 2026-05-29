# Install and update

**Updated:** 2026-05-29

DeepSeek-Mobile is distributed as **source + local build**. The Android APK is **not** stored in git (too large, changes every commit). After each `git pull`, rebuild and reinstall the APK on the phone — user data in `files/deepseek-mobile/` is kept (`adb install -r`).

## One command — Windows PC (toolchain + repo)

**First install** (Git, Rust MSVC, clone/update repo, `cargo check` / tests):

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
$u = 'https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'
$s = "$env:TEMP\setup-pc-windows.ps1"
Invoke-WebRequest $u -OutFile $s
powershell -ExecutionPolicy Bypass -File $s
```

**Update repo only** (inside an existing checkout):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1
```

Update repo **and** rebuild/install APK on phone:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1 -PhoneApk -Serial RFCNC0PWD4E -Launch
```

With workspace check after pull:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1 -Check
```

If you have local changes, commit/stash first, or use `-AllowDirty` (see script help).

Details: [`INSTALL_PC_WINDOWS.md`](./INSTALL_PC_WINDOWS.md).

## One command — full update (git + tests + phone APK)

```powershell
cd $HOME\DeepSeek-Mobile
.\scripts\update-all.ps1 -Serial <adb-serial> -Launch
```

Optional PC Host host binaries and Windows mDNS firewall (run PowerShell as Administrator):

```powershell
.\scripts\update-all.ps1 -Serial RFCNC0PWD4E -Launch -BuildPcHostBundle -EnableMdnsFirewall
```

## One command — Android APK only (build + install on phone)

Requires: USB debugging, repo-local SDK (`. .\tools\android\env.ps1`), `dx` 0.7.x, Rust Android targets.

**First install or upgrade** (same command — reinstalls over the old app, keeps chats/settings on device):

```powershell
cd $HOME\DeepSeek-Mobile   # or your clone path
. .\tools\android\env.ps1
.\scripts\update-phone-apk.ps1 -Serial <adb-serial> -Launch
```

Example:

```powershell
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Launch
```

**Update source + APK in one step:**

```powershell
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Pull -Launch
```

Faster iteration (skip tests, rebuild only if you changed Rust/UI):

```powershell
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -SkipTests
```

Install existing APK without rebuild:

```powershell
.\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -SkipBuild -Launch
```

APK output path (after `dx build`):

```text
target/dx/deepseek-mobile/debug/android/app/app/build/outputs/apk/debug/app-debug.apk
```

Alternative helper (install/launch/screenshots): [`scripts/adb-control.ps1`](../scripts/adb-control.ps1) `-Action InstallLaunch`.

## What “initial APK” means

| Item | In git? | Notes |
|------|---------|--------|
| Source, scripts, `tools/android/` SDK layout | Yes | Clone or `setup-pc-windows.ps1` |
| Prebuilt `app-debug.apk` | **No** | Built on your PC with `dx build` |
| Phone app data | On device | Survives `adb install -r` upgrades |

Every feature merge (PC Host pairing, manual URL, probes, etc.) reaches the phone only after **`update-phone-apk.ps1`** (or CI artifact, when release pipeline exists).

## GitHub Releases (signed APK, optional)

1. Copy `android/keystore.properties.example` to `android/keystore.properties` and create a release keystore (keep secrets out of git).
2. Build and copy to `dist/`:

```powershell
.\scripts\build-release-apk.ps1
```

3. Publish (requires `gh auth login`):

```powershell
.\scripts\publish-github-release.ps1 -NotesFile RELEASE_NOTES.md
```

CI also builds on tag push `v*` (`.github/workflows/release.yml`) when repository secrets `ANDROID_KEYSTORE_*` are set.

**Windows firewall (PC Host LAN):** run from repo root, not `C:\Windows\System32`:

```powershell
cd C:\Users\vladi\Desktop\DeepSeek-Mobile
.\scripts\enable-pc-host-mdns-windows.ps1
```

Or right-click **`enable-pc-host-firewall.cmd`** in the repo root → Run as administrator.

Asset name on the release: `deepseek-mobile-<version>.apk` (matches in-app update check).

## In-app update (Settings)

On the phone: **Settings → App update** — checks `https://api.github.com/repos/Vladrus39/DeepSeek-Mobile/releases/latest`, downloads the APK into app storage, then **Install update** (allow “Install unknown apps” when Android prompts).

For day-to-day dev builds, `update-phone-apk.ps1` remains faster than OTA.

## After install / update

1. Phone: [`DEVICE_SETUP.md`](./DEVICE_SETUP.md) — API key, Termux, workspace path.
2. PC Host (optional): [`PC_HOST_E2E.md`](./PC_HOST_E2E.md), [`INSTALL_PC_WINDOWS.md`](./INSTALL_PC_WINDOWS.md).
3. Smoke: `.\scripts\device-smoke.ps1 -Serial <serial>` (does not rebuild APK).

## Full dev pipeline (optional)

From repo root, logs under `target/verify/`:

```powershell
.\full-rebuild-verify.ps1
```

Runs `cargo test`, `dx build`, `adb install`, launch smoke. Adjust device serial inside the script or use `update-phone-apk.ps1` for day-to-day work.

## What installers do not provide

- Android emulator images (large; use a physical device).
- DeepSeek API key (`.env` for debug builds or enter in app).
- Play Store AAB listing (release APK + sideload OTA exist; store submission is separate).

## Git status reminder

Documentation and scripts in **your local clone** are only “on GitHub” after `git commit` and `git push`. See `git status` before assuming remote `main` matches your machine.
