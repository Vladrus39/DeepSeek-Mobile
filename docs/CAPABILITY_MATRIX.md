# Capability matrix (user-facing)

**Updated:** 2026-05-26

This is what DeepSeek-Mobile **actually** does today. Product stance: **phone-first full agent** ([PRODUCT_POSITIONING.md](./PRODUCT_POSITIONING.md)); PC Host is optional.

## Summary

| Expectation | Supported? |
|-------------|------------|
| **Full coding agent on phone** (TUI-like: files, patch, git, tests) | **Yes** with **Termux workspace** configured |
| Coding agent in **sandbox only** (no Termux) | **Partial** — no `exec_shell`; edits/ZIP/plan only |
| **PC workstation** (optional boost) | **Yes** when paired — not required for “full agent” |
| **Phone-only** without Termux setup | **Lite** — not equivalent to desktop TUI |
| Control **entire phone** (all apps, system UI) | **Partial** — `phone_control` (URL, share, launch app by package); not full UI automation |
| Control **entire PC** (any folder, any app) | **Partial** — paired workspace + optional trusted paths (Settings grant mode); see [EXTENDED_CONTROL_ROADMAP.md](./EXTENDED_CONTROL_ROADMAP.md) |
| Same as **Cursor on desktop** | **No** — different product shape |

## By channel

### Phone sandbox (LocalAndroid workspace)

| Capability | Status |
|------------|--------|
| Chat + streaming | Yes |
| read/write/patch files in app workspace | Yes |
| exec_shell | **No** — use Termux or PC (clear error message) |
| git/rust build in sandbox | Only if tools exist in sandbox (not assumed) |
| Snapshots | Yes |
| Import/export project ZIP | Yes (Android picker on device) |

### Termux workspace (primary — full agent on phone)

| Capability | Status |
|------------|--------|
| exec_shell (approved) | Yes (native bridge + model continuation) |
| File tools under Termux path | Yes |
| git / build / test | Yes if installed in Termux |
| Requires Termux + RUN_COMMAND + path in Settings | User setup (onboarding helps) |

### PC Host workspace (optional — huge repos)

| Capability | Status |
|------------|--------|
| Files, patch, shell stream | Yes when host running |
| Git panel + engine git tools | Yes |
| Terminal / tasks / SSE | Yes |
| Requires pairing + `deepseek-pc-host` on PC | **Optional** user setup |

### Always on phone (any active workspace)

| Capability | Status |
|------------|--------|
| github_* tools | Yes (token from settings) |
| web_fetch / web_search | Yes |
| MCP proxy tools | Yes (when servers connected) |
| Plan mode | Yes — **tools not executed** |

## Product features added for clarity

- **Setup wizard** (3 steps): API → backends explained → cockpit
- **Health panel**: API, PC, Termux, MCP, native bridge + recommendations
- **Quick actions** in chat: plan, git status, tests, structure, diagnostics
- **Isolated Android SDK**: `tools/android/` (no mix with other projects)

## When internet returns

See `tools/android/DOWNLOAD_BUDGET.md` (~1.0–1.2 GB for APK on a real phone).
