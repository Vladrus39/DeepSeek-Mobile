# PC Host — E2E and pairing

**Phase 4** (optional): offload large repos / desktop toolchains to a PC while the phone stays the cockpit.

## One-click pairing from the Android app (recommended on Windows)

1. On the phone: **PC Host** → configure pairing → **Export pairing ZIP** (share to PC or copy via USB).
2. On the PC: unzip the bundle anywhere (e.g. `Downloads\DeepSeek-PC-Host`).
3. Double-click **`Setup-DeepSeek-PC-Host.cmd`** (or run `Setup-DeepSeek-PC-Host.ps1`).
4. Approve the **one-time UAC** prompt — the setup script adds Windows Firewall rules for the gateway port (default TCP **8787**) and mDNS (UDP **5353**) on **Private** networks only.
5. Setup registers a **logon scheduled task** (`DeepSeekPcHost`), starts the host, and creates the **`Project workspace`** folder next to the bundle.

The ZIP contains:

| File | Purpose |
|------|---------|
| `Setup-DeepSeek-PC-Host.cmd` / `.ps1` | One-click install: firewall + autostart + first launch |
| `start-deepseek-pc-host.ps1` | Manual launch (no firewall/task setup) |
| `deepseek-pc-host.env` | Bind address, token, workspace id |
| `pairing.json` | Full pairing metadata for the app |
| `README.txt` | Short user instructions |
| `deepseek-pc-host.exe` | Embedded when the app found a build at export time |

**Project workspace:** PC projects live in a sibling folder named **`Project workspace`** (created beside the unzipped bundle). The phone's local sandbox uses the same folder name under `files/deepseek-mobile/`.

**Nuances:**

- If the ZIP has **no** embedded `deepseek-pc-host.exe`, run `.\scripts\build-pc-host-bundles.ps1` on a dev machine, re-export from the phone, or install `deepseek-pc-host` on PATH before setup.
- Setup does **not** require cloning this repo or running scripts from `C:\Windows\System32`.
- After setup, return to the phone: **Scan LAN (mDNS)** or **Connect manually** with `http://<PC-LAN-IP>:8787`.
- Tool calls that touch paths **outside** the project workspace require user approval in chat; approving grants that path on the PC host for the session (see **Security** below).

For manual/dev bootstrap from a git checkout, see [`INSTALL_PC_WINDOWS.md`](./INSTALL_PC_WINDOWS.md).

## Requirements

1. **Same LAN / same subnet** — phone and PC must share a Wi‑Fi network (e.g. both `192.168.1.x`).  
   If the phone is `172.18.x.x` and the PC is `192.168.x.x`, discovery will **fail** (different networks).
2. **PC Host listening on all interfaces** for mDNS:

```powershell
cd C:\Users\vladi\Desktop\DeepSeek-Mobile
$env:DEEPSEEK_PC_HOST_BIND = "0.0.0.0:8787"
$env:DEEPSEEK_PC_HOST_WORKSPACE = (Get-Location).Path
cargo run -p deepseek-pc-host
```

3. **Windows Firewall + mDNS** — allow inbound TCP **8787** and UDP **5353** on private networks:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\enable-pc-host-mdns-windows.ps1
```

Or during PC bootstrap:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-pc-windows.ps1 -SkipTests -EnableMdnsFirewall -OpenFirewall -PcHostBind '0.0.0.0:8787'
```

4. **Phone** — USB debugging, app installed, `CHANGE_WIFI_MULTICAST_STATE` granted (manifest).

## Verify mDNS on Windows

After starting the host, the console must show a line like:

```text
deepseek-pc-host mDNS: DeepSeek-Mobil on 192.168.1.111:8787 (_deepseek-pc-gateway._tcp.local.)
```

If you only see `mDNS advertise failed`, run the firewall script above and restart the host. Manual URL in the app still works as fallback.

## Automated E2E

**Full pairing bundle (APK export → unzip on PC → launch script):**

```powershell
. .\tools\android\env.ps1
.\scripts\device-e2e-pc-pairing-bundle.ps1 -Serial RFCNC0PWD4E
```

**Discovery only** (PC host already running):

```powershell
. .\tools\android\env.ps1
.\scripts\device-e2e-pc-host.ps1 -Serial RFCNC0PWD4E
```

Steps (discovery script): subnet check → PC `/health` → phone mDNS probe → writes  
`files/deepseek-mobile/.pc_discovery_probe_result` (`PASS endpoints=…`).

Full suite (includes PC host when not skipped):

```powershell
.\scripts\device-full-verify.ps1 -Serial RFCNC0PWD4E -SkipBuild
```

## How the phone connects (LAN vs internet)

| Mode | When to use | In the app |
|------|-------------|------------|
| **Auto (LAN)** | Phone and PC on the **same Wi‑Fi / subnet** | **PC Host** → **Scan LAN (mDNS)** — finds `_deepseek-pc-gateway._tcp` |
| **Manual (LAN)** | mDNS blocked or you know the PC IP | Enter `http://192.168.x.x:8787` → **Connect manually** |
| **Manual (internet)** | PC reachable via **HTTPS** (Tailscale, Cloudflare Tunnel, VPS, reverse proxy) | Enter `https://your-host` → **Connect manually** |

Core transport policy: **HTTP only on private/loopback** addresses; **public hosts require HTTPS** (`InternetHttps` / `TunnelHttps`). Plain `http://` to a public hostname is rejected.

## Manual pairing (UI)

1. Start `deepseek-pc-host` on the PC (see above), **or** use the pairing ZIP **`Setup-DeepSeek-PC-Host.cmd`** flow.
2. On the phone: **PC Host** tab:
   - **Scan LAN (mDNS)** for automatic discovery, or
   - type base URL and **Connect manually** (LAN or remote HTTPS).
3. Pick / apply the best online route from the discovery list if needed.
4. Optional: export another pairing ZIP for a second PC (same one-click setup).

## Trusted paths (chat approval grant)

By default the PC host serves only the **Project workspace** root from the pairing bundle.

When the agent proposes a file/shell tool on an **absolute path outside** that root (e.g. `D:\OtherRepo\src\main.rs`), the phone shows an approval dialog. **Approving** calls `grant_trusted_path` on the PC gateway so that path (or its parent directory) is allowed for the rest of the host session. Paths synced from **Settings → trusted external paths** are included in the pairing ZIP as `DEEPSEEK_PC_HOST_TRUSTED_PATHS`.

This is intentional: the phone cockpit stays in control; the PC does not expose the whole disk by default.

## Security model (accurate, not a VPN)

DeepSeek PC Host is **not** an encrypted VPN tunnel between phone and PC.

| Layer | Behavior |
|-------|----------|
| Transport | HTTP/WebSocket gateway on LAN by default (`LocalNetworkHttp`). Traffic on Wi‑Fi is **not encrypted** unless you configure **HTTPS** (`LocalNetworkHttps` / `TunnelHttps` for remote hosts). |
| Authentication | Pairing **token** in the ZIP (`Authorization: Bearer …`). Every gateway request must present it when the host has a token set. |
| Exposure | Host binds to `0.0.0.0:8787` for LAN discovery. **Anyone on the same LAN who knows IP, port, and token** can call the gateway — same trust model as a shared API key on a home network. |
| Firewall | Windows setup adds inbound rules for the gateway port and mDNS on **Private** profile only; Public networks are not opened by default. |
| Scope | Workspace-scoped file/git/shell tools; path traversal rejected; extra directories only via explicit trusted-path grants or pairing metadata. |
| Secret handling | Treat the pairing ZIP like a password — do not email or share publicly. Regenerate by exporting a new bundle from the phone if leaked. |

There is no claim of "military-grade" encryption on the default LAN path. For untrusted networks, use HTTPS termination (Tailscale, reverse proxy, Cloudflare Tunnel) and rotate tokens.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `FAIL timeout` discovery | Same Wi‑Fi; host on `0.0.0.0:8787`; firewall; reopen app. Windows often blocks mDNS — use **Connect manually** in the app or E2E script fallback (`.pc_discovery_manual_url`) |
| `0 candidates` | mDNS blocked — check router AP isolation / guest network |
| Stuck `.pc_discovery_probe_running` | Reboot app or `adb shell run-as com.deepseek.mobile rm files/deepseek-mobile/.pc_discovery_probe_running` |
| Health OK on PC but phone fails | Subnet mismatch — run script to see phone vs PC IPs |

## Token (optional)

```powershell
$env:DEEPSEEK_PC_HOST_TOKEN = "your-shared-secret"
```

Configure the same token in the app pairing flow when supported.
