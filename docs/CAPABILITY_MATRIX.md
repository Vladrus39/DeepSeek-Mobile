# Capability matrix (user-facing)

**Updated:** 2026-05-26

Product stance: **phone-first full agent** ([PRODUCT_POSITIONING.md](./PRODUCT_POSITIONING.md)); PC Host is optional. Current factual status is tracked in [CURRENT_STATE.md](./CURRENT_STATE.md).

## Summary

| Expectation | Supported? |
|---|---|
| Full coding agent on phone | **Target path is implemented through Termux**, but final Termux hardware callback verification remains before v1 |
| Coding agent in sandbox only | **Partial** — files/patch/ZIP/plan; no normal shell executor |
| PC workstation backend | **Yes** when paired; optional, not required for product positioning |
| Phone-only without Termux | **Lite mode** — not equivalent to desktop TUI |
| Control entire phone | **Partial** — URL/share/settings/launch-app hooks; not full UI automation |
| Control entire PC | **Partial** — paired workspace and PC Host protocol, not arbitrary remote desktop |
| Same as Cursor desktop | **No** — different product shape |

## By channel

### Phone sandbox: LocalAndroid workspace

| Capability | Status |
|---|---|
| Chat + streaming | Implemented |
| Read/write/patch files in app workspace | Implemented |
| Snapshots | Implemented |
| Project ZIP import/export | Implemented; Android picker/share hardware verification pending |
| `exec_shell` | Not supported in sandbox; use Termux or PC Host |
| Git/build/test | Only if tools exist in the active executor; not assumed in sandbox |

### Termux workspace: primary full-agent phone path

| Capability | Status |
|---|---|
| Save/activate Termux workspace path | Implemented |
| Queue approved `exec_shell` through native bridge | Implemented |
| Termux `RUN_COMMAND` intent and result parser | Implemented |
| Continue model turn from stdout/stderr/exit code | Implemented in Rust/mobile path |
| Real hardware happy-path verification | Pending |
| Requires Termux permission and `allow-external-apps=true` | User setup |

### PC Host workspace: optional large-repo path

| Capability | Status |
|---|---|
| Files, patch, shell stream | Implemented when host is running |
| Git panel + engine git tools | Implemented |
| Terminal / tasks / SSE | Implemented |
| Diagnostics | Rust/TypeScript/Python implemented |
| Requires pairing + `deepseek-pc-host` | Optional user setup |
| Release packaging/autostart | Pending |

### Always available from the phone UI

| Capability | Status |
|---|---|
| Onboarding/settings | Implemented |
| Approvals | Implemented |
| GitHub tools | Implemented; token from settings/environment |
| Web tools | Implemented |
| MCP registry/UI/proxy surfaces | Implemented partially |
| External MCP execution | Pending hardening |
| Plan mode | Implemented; tools are not executed |

## Android status

| Item | Status |
|---|---|
| Debug APK build | Verified with `dx build --android --package deepseek-mobile --device RFCNC0PWD4E` |
| Install/launch on physical phone | Verified |
| Android UI render | Verified: onboarding/cockpit render path works |
| Android icon/favicon | Implemented |
| Full native flow verification | Pending |

## User-facing interpretation

- **Full agent ready on phone** should mean: API key configured + valid Termux workspace + Termux permission/setup verified.
- **PC boost ready** should mean: PC Host paired and active.
- **Sandbox ready** should mean: chat/files/ZIP/snapshots are available, but shell/build/test are not promised.
