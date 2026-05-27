# Install and update

**Updated:** 2026-05-26

This project is still a source checkout + debug Android build workflow. The commands below are intentionally conservative: they do not download an Android emulator or overwrite local changes.

## Windows: install with one command

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/install-windows.ps1 | iex"
```

Default checkout path:

```text
%USERPROFILE%\DeepSeek-Mobile
```

Custom path:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "$s=irm https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/install-windows.ps1; & ([scriptblock]::Create($s)) -Dir 'D:\DeepSeek-Mobile'"
```

## Windows: update with one command

From inside an existing checkout:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1
```

With a Rust workspace check after update:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1 -Check
```

If local files are modified, the updater stops instead of overwriting them. Commit/stash the changes first, or explicitly pass `-AllowDirty` if you understand the risk.

## After install/update

```powershell
cd $HOME\DeepSeek-Mobile
. .\tools\android\env.ps1
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
```

With a USB-debugging phone connected:

```powershell
dx build --android --package deepseek-mobile --device <serial> --verbose
```

## What this does not install

- Android emulator / system images — too large for limited daily traffic.
- Real DeepSeek API key — use `.env` for local debug builds or enter it in the app.
- Release signing keys — release packaging is a separate step.
