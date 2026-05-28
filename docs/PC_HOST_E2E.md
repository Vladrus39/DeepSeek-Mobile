# PC Host — E2E and pairing

**Phase 4** (optional): offload large repos / desktop toolchains to a PC while the phone stays the cockpit.

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

1. Start `deepseek-pc-host` on the PC (see above).
2. On the phone: **PC Host** tab:
   - **Scan LAN (mDNS)** for automatic discovery, or
   - type base URL and **Connect manually** (LAN or remote HTTPS).
3. Pick / apply the best online route from the discovery list if needed.
4. Optional: export pairing ZIP for another PC (`install-pc-host-from-pairing.ps1`).

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
