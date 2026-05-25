# Capability matrix (user-facing)

**Updated:** 2026-05-26

This is what DeepSeek-Mobile **actually** does today — compared to a desktop IDE agent (Cursor) and to “control the whole phone/PC”.

## Summary

| Expectation | Supported? |
|-------------|------------|
| Coding agent on a **project** (files, patch, git, tests via tools) | **Yes** (with PC Host or Termux for shell) |
| **PC workstation** remote control via pairing | **Yes** (workspace-bound, token auth) |
| **Phone-only** sandbox editing + Termux shell | **Yes** (Termux needs setup) |
| Control **entire phone** (all apps, system UI) | **No** |
| Control **entire PC** (any folder, any app) | **No** — only paired workspace root |
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

### Termux workspace

| Capability | Status |
|------------|--------|
| exec_shell (approved) | Yes (native bridge + model continuation) |
| File tools under Termux path | Yes |
| Requires Termux app + RUN_COMMAND permission | User setup |

### PC Host workspace

| Capability | Status |
|------------|--------|
| Files, patch, shell stream | Yes |
| Git panel + engine git tools | Yes |
| Diagnostics reinjection | Yes |
| Terminal / tasks / SSE task events | Yes |
| Requires running `deepseek-pc-host` | User setup |

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
