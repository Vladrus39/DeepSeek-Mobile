# PC Host release binaries (local, optional)

Place or build `deepseek-pc-host` here so the Android pairing ZIP can embed the binary offline.

## Build into this folder

From the repo root:

```powershell
.\scripts\build-pc-host-bundles.ps1
```

Expected layout after build:

- `windows-x86_64/deepseek-pc-host.exe`
- `linux-x86_64/deepseek-pc-host` (when built on Linux/WSL)
- Flat copies at repo root of this folder for convenience

Binaries are **not** committed to git (see `.gitignore`). CI and developers run the script before exporting a pairing bundle from the phone.

## Discovery order

`discover_pc_host_binaries` checks, per workspace root:

1. `tools/pc-host/bin/windows-x86_64/deepseek-pc-host.exe`
2. `tools/pc-host/bin/deepseek-pc-host.exe`
3. `target/release/deepseek-pc-host.exe`

Unix hosts use the parallel paths without `.exe`.
