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

3. **Windows Firewall** — allow inbound TCP **8787** on private networks (first run may prompt).

4. **Phone** — USB debugging, app installed, `CHANGE_WIFI_MULTICAST_STATE` granted (manifest).

## Automated E2E

```powershell
. .\tools\android\env.ps1
.\scripts\device-e2e-pc-host.ps1 -Serial RFCNC0PWD4E
```

Steps: subnet check → PC `/health` → phone mDNS probe → writes  
`files/deepseek-mobile/.pc_discovery_probe_result` (`PASS endpoints=…`).

Full suite (includes PC host when not skipped):

```powershell
.\scripts\device-full-verify.ps1 -Serial RFCNC0PWD4E -SkipBuild
```

## Manual pairing (UI)

1. Start `deepseek-pc-host` on the PC (see above).
2. On the phone: **PC Host** tab → **Scan** / discovery.
3. Pick the discovered endpoint → connect workspace.
4. Optional: export pairing ZIP for another PC (`install-pc-host-from-pairing.ps1`).

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `FAIL timeout` discovery | Same Wi‑Fi; host on `0.0.0.0:8787`; firewall; reopen app |
| `0 candidates` | mDNS blocked — check router AP isolation / guest network |
| Stuck `.pc_discovery_probe_running` | Reboot app or `adb shell run-as com.deepseek.mobile rm files/deepseek-mobile/.pc_discovery_probe_running` |
| Health OK on PC but phone fails | Subnet mismatch — run script to see phone vs PC IPs |

## Token (optional)

```powershell
$env:DEEPSEEK_PC_HOST_TOKEN = "your-shared-secret"
```

Configure the same token in the app pairing flow when supported.
