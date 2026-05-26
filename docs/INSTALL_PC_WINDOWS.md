# DeepSeek-Mobile — Windows PC install and update

This guide is for Windows users who want to run DeepSeek-Mobile on a PC and/or use the PC as a `deepseek-pc-host` backend for the Android app.

The recommended entry point is the bootstrap script:

```text
scripts/setup-pc-windows.ps1
```

It is designed for both first installation and later updates.

## What the bootstrap script does

On every run it:

1. Checks/install Git through `winget` if missing.
2. Checks/install Rustup through `winget` if missing.
3. Installs/updates the stable MSVC Rust toolchain.
4. Checks/install Visual Studio 2022 Build Tools with MSVC if missing.
5. Checks/install Microsoft Edge WebView2 Runtime if missing.
6. Clones the repository if it is not present.
7. Updates the existing repository with `git fetch`, `git checkout main`, `git pull --ff-only` if it is already present.
8. Creates `.env` only if it does not exist.
9. Runs `cargo check --workspace --all-targets`.
10. Runs `cargo test --workspace` unless `-SkipTests` is provided.
11. Optionally builds the PC Host release binary bundle.
12. Optionally opens the Windows Firewall rule for PC Host LAN access.
13. Optionally installs PC Host autostart.
14. Optionally starts PC Host in the current PowerShell window.

The script does **not** overwrite an existing `.env`.

## Requirements

Use Windows 10/11 with PowerShell.

`winget` must be available. If it is missing, install **App Installer** from Microsoft Store, reopen PowerShell, and run the command again.

For automatic system installation, run PowerShell as Administrator. Non-admin mode may still work when all required tools are already installed.

## First install — one command

Open PowerShell as Administrator and run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s
```

This installs missing PC dependencies, clones the project into:

```text
%USERPROFILE%\DeepSeek-Mobile
```

and runs workspace check/tests.

## First install with DeepSeek API key

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s -DeepSeekApiKey 'sk-your-key-here'
```

If `.env` already exists, it is preserved and not overwritten.

## Update to latest version — one command

Use the same bootstrap script again. It detects the existing repo and updates it instead of cloning again:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s
```

Faster update without running full tests:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s -SkipTests
```

## Run the desktop UI

After setup:

```powershell
cd $HOME\DeepSeek-Mobile
cargo run -p deepseek-mobile
```

The desktop binary is for development/testing. Android uses the mobile library target during APK builds.

## Run PC Host manually

For local-only testing:

```powershell
cd $HOME\DeepSeek-Mobile
$env:DEEPSEEK_PC_HOST_BIND='127.0.0.1:8787'
$env:DEEPSEEK_PC_HOST_WORKSPACE=$PWD
$env:DEEPSEEK_PC_HOST_TOKEN='123456789'
cargo run -p deepseek-pc-host
```

Check health from another PowerShell window:

```powershell
Invoke-RestMethod http://127.0.0.1:8787/health
```

## Run PC Host for phone access over LAN

Run setup with firewall opening and a LAN bind address:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s -SkipTests -OpenFirewall -StartPcHost -PcHostBind '0.0.0.0:8787' -PcHostToken '123456789'
```

Find the PC IP address:

```powershell
ipconfig
```

On the phone, use:

```text
http://PC_IP_ADDRESS:8787
```

Token:

```text
123456789
```

## Build PC Host release bundle

```powershell
cd $HOME\DeepSeek-Mobile
.\scripts\setup-pc-windows.ps1 -SkipTests -BuildPcHostBundle
```

This builds `deepseek-pc-host.exe` and copies it to `tools/pc-host/bin/` for pairing/release bundle embedding.

## Install PC Host autostart

Run PowerShell as Administrator:

```powershell
cd $HOME\DeepSeek-Mobile
.\scripts\setup-pc-windows.ps1 -SkipTests -BuildPcHostBundle -InstallAutostart -OpenFirewall -PcHostBind '0.0.0.0:8787' -PcHostToken '123456789'
```

This creates a Windows Scheduled Task named `DeepSeekPcHost`.

## Custom install folder

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force; $u='https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/setup-pc-windows.ps1'; $s="$env:TEMP\setup-pc-windows.ps1"; Invoke-WebRequest $u -OutFile $s; powershell -ExecutionPolicy Bypass -File $s -RepoDir 'D:\DeepSeek-Mobile'
```

Use the same `-RepoDir` value later for updates.

## Notes

- If `git pull --ff-only` fails, the local repository has changes that cannot be fast-forwarded safely. Commit/stash them or install to another `-RepoDir`.
- If MSVC was installed during the run and Cargo still fails to find the linker, reboot Windows or reopen PowerShell and rerun the bootstrap command.
- `.env` is local and gitignored. It is not overwritten by the bootstrap script.
