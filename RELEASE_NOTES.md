# DeepSeek-Mobile — Release Notes

## Build 2026-05-29 — PC Host pairing, Windows mDNS, MCP reuse

### New

- **PC Host pairing UI**: manual URL (`http://LAN:8787` / `https://tunnel`), LAN mDNS scan, pairing ZIP export + Android share.
- **Windows mDNS fix**: PC host keeps mDNS daemon alive; `scripts/enable-pc-host-mdns-windows.ps1` for firewall (TCP 8787, UDP 5353).
- **Phone discovery fallback**: E2E/manual URL file when mDNS is blocked on Windows LAN.
- **MCP stdio reuse**: long-lived child per server, reconnect on invoke failure, panel disconnect.
- **MCP approvals**: `mcp__server__tool` calls require user approval; execution validates enabled server + known tool in `mcp.json`.
- **Update scripts**: `update-phone-apk.ps1`, `update-all.ps1` (git pull + test + APK install).

### Commands

```powershell
.\scripts\update-all.ps1 -Serial <adb-serial> -Launch
.\scripts\enable-pc-host-mdns-windows.ps1   # once, admin
```

### Release pipeline (2026-05-29 follow-up)

- **Release APK**: `scripts/build-release-apk.ps1` → `dist/deepseek-mobile-<version>.apk` (optional `android/keystore.properties`).
- **GitHub Releases**: `scripts/publish-github-release.ps1`; CI on tag `v*` (`.github/workflows/release.yml`).
- **In-app update**: Settings → App update (GitHub latest release check, download, sideload install).

### Still pending

- Play Store AAB submission.
- Full manual device checklist (picker, ZIP UI, Termux on hardware).

## Build 2026-05-26 — PC Task Reconciliation

### New

- **PC running-task reconciliation in Tasks panel**: the mobile Tasks panel now syncs active PC-host running tasks through `PcGatewayClient::list_tasks()`, shows them separately from local durable records, reconciles duplicate local/PC task ids in the nav badge count, and can send `StopTask` for active PC tasks.

### Tests

- Full workspace: 128 mobile / 166 core / 2 pc-host tests — all green.
- Added mobile task-state tests for PC running-task sorting, local/PC active-count reconciliation and clearing PC sync state.

### Build

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets`: green, with existing dead-code warnings in mobile test-only/native bridge surfaces.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace`: 296 tests pass.

### Still pending

- Runtime SSE/live event streaming and automatic task updates.
- Final Android/Dioxus host adapter and device/emulator verification.
- Final Android/Dioxus host picker/share device verification for project import/export.

## Build 2026-05-25 — Artifacts, Logs, Runtime API, Termux Workspace & Workspace IO

### New

- **Durable task artifacts and logs**: `DurableTaskManager` supports `add_artifact()`, `append_log()`, and `read_log()`. Task log files are stored under `{data_dir}/logs/{task_id}.log` with Unix timestamps, and the log path is tracked in `artifact_paths`.
- **PC-host background task logging**: `RunTask` pipes stdout/stderr into per-task logs under `.deepseek-mobile/tasks/logs/`.
- **Runtime HTTP task API on PC-host**:
  - `GET /v1/runtime/tasks` — returns currently running tasks with id, label, kind and started timestamp.
  - `GET /v1/runtime/tasks/{task_id}/log` — returns the task log content.
- **Artifact display in Tasks panel**: task cards show artifact count and the first artifact path when present.
- **Termux workspace selector**: Settings now has a Termux workspace section. Saving a valid absolute Termux path persists `termux_workspace.json` and activates a `WorkspaceConnection` so future mobile turns run against the Termux backend.
- **Termux path hardening**: the selector rejects empty paths, relative paths, parent-directory traversal, Windows-style paths/backslashes and invalid saved configs on reload.
- **Core workspace ZIP import/export helpers**: `workspace_io` can import/export project ZIPs, rejects unsafe archive paths, emits portable ZIP entry names and excludes `.deepseek-mobile` metadata from exports.
- **Files panel project import/export UI**: the Files panel now has Import ZIP and Export ZIP controls for the local phone workspace. Import queues the Android archive picker and extracts the returned local archive copy; export creates a ZIP under `.deepseek-mobile/exports/` and queues native share.
- **Document picker purpose routing**: Android picker callbacks now route project-import archives to the project transfer flow while chat attachments still go into the composer. Native share callbacks update only active project exports.

### Tests

- Full workspace: 125 mobile / 166 core / 2 pc-host tests — all green for this build.
- Added Termux workspace state tests for activation, strict path validation and invalid saved-config revalidation.
- Added workspace import/export tests for ZIP traversal rejection, Windows/absolute path rejection, metadata exclusion and export/reimport roundtrip.
- Added project transfer state tests for import request state, missing local archive rejection, ZIP extraction, export ZIP creation and default phone workspace/export paths.
- Durable task artifact/log coverage is included in the current core count.

### Build

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets`: green, with existing dead-code warnings in mobile test-only/native bridge surfaces.
- `cargo +stable-x86_64-pc-windows-msvc test --workspace`: 293 tests pass.
- PC-host depends on `tracing = "0.1"` for task log capture.

### Still pending

- Runtime SSE/live event streaming and automatic task updates.
- Final Android/Dioxus host adapter and device/emulator verification.
- Final Android/Dioxus host picker/share device verification for project import/export.
