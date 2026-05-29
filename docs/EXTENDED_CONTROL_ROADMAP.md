# Extended control roadmap (phone-first)

**Canonical positioning:** [PRODUCT_POSITIONING.md](./PRODUCT_POSITIONING.md)

This plan extends the **phone-first** agent (Termux = full executor on device). PC Host is **optional boost** for oversized projects, not the primary product path.

## Executors (priority order)

```mermaid
flowchart TB
  Agent[Mobile agent UI + core]
  T[Termux — primary full agent on phone]
  S[App sandbox — lite edits / ZIP]
  P[PC Host — optional huge-repo boost]
  Agent --> T
  Agent --> S
  Agent --> P
```

| Priority | Executor | Purpose |
|----------|----------|---------|
| 1 | **Termux** | Shell, git, build, test — **same role as desktop terminal in TUI** |
| 2 | **Local sandbox** | Safe storage, attachments, small edits without Termux |
| 3 | **PC Host** | When repo/tooling is too heavy for phone; finish work on workstation |
| — | **phone_control** | URLs, share sheet, launch app, settings — auxiliary |

## Phase A — Phone-first v1 (current focus)

1. **Termux happy path** — onboarding saves path, Settings validation, Health “Full agent ready”, device verification of RUN_COMMAND bridge. Android startup smoke is verified; Termux callback verification remains.
2. **TUI parity gaps on device** — MCP stdio on-device verification, file summaries / symbol search (see ROADMAP Phase 7–8). Large-output routing and multi-round tool follow-up are implemented in core.
3. **Default workspace policy** — `PreferTermux` for new installs; PC does not override Termux unless user activates PC workspace.
4. **phone_control** — `open_url`, `share_file`, `launch_app`, `open_settings`. *Shipped.*

## Phase B — Optional PC boost (already partially shipped)

Use only when the user opens **PC Host** panel or activates a PC workspace:

- Pairing ZIP + embedded `deepseek-pc-host`
- One-click Windows setup (`Setup-DeepSeek-PC-Host.cmd` / `.ps1`: firewall UAC once, logon scheduled task)
- **`Project workspace`** folder beside the unzipped bundle (same name as phone sandbox subfolder)
- Trusted paths (`DEEPSEEK_PC_HOST_TRUSTED_PATHS`) + **grant on chat approval** for paths outside workspace
- `open_path` in OS file manager on PC
- Tasks / SSE / terminal on pc-host

**Not** the default onboarding message (“you must pair PC to be pro”).

## Phase C — Later (explicitly not v1 core)

- Accessibility / Shizuku / ADB UI automation (opt-in, policy risk)
- Cloud relay / tunnel as default (LAN pairing is enough for v1)
- MCP proxy **only on PC** — nice-to-have; prefer **on-device MCP** first for phone-first parity

## Security (LAN gateway, not VPN)

1. Pairing token in the ZIP = shared secret; optional expiry in bundle metadata.
2. Default transport on LAN is **plain HTTP** — not end-to-end encrypted unless HTTPS is configured.
3. Same-LAN actors with token + IP:port can reach the gateway; firewall limits profile to Private networks on Windows setup.
4. Workspace root + explicit trusted paths only; chat approval grants extra paths per session.
5. Grants visible in Settings (trusted paths synced into pairing export).
6. Plan mode never runs tools.
7. Approvals for shell/write/network.

## How to verify phone-first locally

1. Install Termux + `allow-external-apps=true` (see `docs/TROUBLESHOOTING.md`).
2. Onboarding or Settings → save valid Termux path → Health shows **Full agent ready**.
3. Agent mode → `exec_shell` with `pwd` / `git status` — timeline continues after Termux callback.
4. PC Host panel — **skip**; agent should still run full tools on Termux project.

## How to verify optional PC boost

1. `.\scripts\build-pc-host-bundles.ps1` (dev machine) so the phone can embed `deepseek-pc-host.exe` in the export.
2. Phone: export pairing ZIP → PC: unzip → double-click **`Setup-DeepSeek-PC-Host.cmd`**.
3. Phone: **Scan LAN** or manual URL → activate PC workspace.
4. Run tests/git under **`Project workspace`** on the PC; approve a tool on an outside path and confirm grant works.
