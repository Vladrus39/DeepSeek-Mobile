# Product positioning (canonical)

**Last updated:** 2026-05-26

## One sentence

**DeepSeek-Mobile is a full DeepSeek coding agent on Android** — a TUI-like agent workflow with the phone as the only required device; **Termux is the primary execution backend** for real projects; **PC Host is optional** for very large repos or when the phone toolchain is not enough.

## What we are building

| Layer | Role |
|-------|------|
| **Android app** | Cockpit: chat, approvals, timeline, files, settings, health |
| **Core (`deepseek-mobile-core`)** | Agent, tools, tool loop, snapshots, git, MCP — TUI-parity runtime direction |
| **Termux** | **Default “full agent” executor** on phone: shell, git, build, tests in a real project directory |
| **Local sandbox** | Lite mode: edits, ZIP import/export, planning — not a substitute for shell |
| **PC Host (optional)** | Remote workstation when the project outgrows the phone or you want desktop toolchains without installing them in Termux |

## What we are not building

- A PC-remote-control or TeamViewer-style app (PC is not the product center).
- Cursor IDE on the whole phone filesystem.
- Mandatory PC pairing to use the agent.

## Parity with DeepSeek-TUI (PC desktop agent)

| TUI capability | On phone without PC |
|----------------|---------------------|
| Engine, streaming, approvals | Yes |
| File tools + patch | Yes (sandbox or Termux path) |
| Shell / git / tests | **Yes via Termux workspace** |
| Snapshots, diagnostics, GitHub | Yes |
| MCP / skills | Partial (stdio on-device still maturing) |
| Large-output routing, repo overview | Yes |
| LSP UI / symbol search | Planned (see ROADMAP) |

## User-facing labels

- **Full agent on phone** = API key + **valid Termux workspace** + Agent mode.
- **Optional PC boost** = paired `deepseek-pc-host` for huge projects; see PC Host panel.
- Health panel **“Full agent ready”** follows Termux, not PC online status.

## Current-status note

This file defines product direction. Current implementation status is tracked in `docs/CURRENT_STATE.md`.

## Doc alignment

These documents must stay consistent with this file:

- `README.md`
- `ROADMAP.md` (Vision section)
- `docs/PHONE_PC_OPERATING_MODEL.md`
- `docs/CAPABILITY_MATRIX.md`
- `docs/EXTENDED_CONTROL_ROADMAP.md`

When adding features, ask: **does this strengthen phone-first + Termux, or is it optional PC boost?** Do not imply PC is required for a “real” agent.
