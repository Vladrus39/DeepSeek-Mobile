# DeepSeek-Mobile troubleshooting

## Build and development

### `cargo check` fails on Windows with GNU toolchain

Use the MSVC toolchain for local verification:

```powershell
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace
```

### Dioxus CLI (`dx`) is missing

Install the Dioxus CLI version that matches `dioxus = 0.7` in the workspace `Cargo.toml`, then run:

```bash
dx serve --platform android

Project-local SDK (isolated from `D:\Project V`):

```powershell
. .\tools\android\env.ps1
```

Download sizes when back online: `tools/android/DOWNLOAD_BUDGET.md`.
```

Until `dx` is installed, Rust/UI logic can still be verified with `cargo test -p deepseek-mobile`.

## Android host integration

### Native commands stay queued

Symptom: timeline shows `Android host action queued: {...}` but picker/Termux/PC discovery never runs.

Cause: the Kotlin shell is not draining `NativeBridgeState` yet.

Fix:

1. Install NDK + `dx` per `tools/android/DOWNLOAD_BUDGET.md`, then verify on device (`dx build android`).
2. Call `DeepSeekMobileHostCoordinator.pollAndHandleNextAction()` on each UI tick.
3. Forward Termux `PendingIntent` results back through `deliverHostCallbackJson`.

See `docs/android_host_integration.md` and `android/bridge/.../DeepSeekMobileHostCoordinator.kt`.

### Termux commands fail immediately

Checklist:

- Termux is installed.
- `com.termux.permission.RUN_COMMAND` is granted.
- `allow-external-apps=true` is set in `~/.termux/termux.properties`.
- Settings → Termux workspace path is an absolute path under `/data/data/com.termux/files/home/...`.

## PC Host

### Phone cannot reach PC Host

1. Confirm `deepseek-pc-host` is running on the PC.
2. Open the pairing ZIP launcher or start the host manually with the workspace root and token.
3. Verify firewall rules for the bind port (default `8787`).
4. Use the PC Host panel → scan/retry route; check endpoint health rows.

### PC tasks do not update live

The Tasks panel subscribes to `stream_task_events` over SSE. If SSE is blocked by a proxy, use **Refresh** to reconcile through `list_tasks`.

## Settings and secrets

API keys and GitHub tokens are stored in encrypted `secrets.enc` with a per-device `device.key` under `.deepseek-mobile/`. If decryption fails after copying data directories between machines, delete `device.key` and `secrets.enc`, then re-enter secrets in Settings.

## MCP servers

- HTTP/SSE servers: use **Connect** in the MCP panel to run `tools/list`.
- Stdio servers: require a host OS process; declare static tools in `mcp.json` under `declared_tools` until stdio spawn is wired on Android.
