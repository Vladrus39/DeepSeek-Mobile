# Capability matrix (user-facing)

**Updated:** 2026-06 (after ideal hygiene + quality pass; software matches full conceived design)

Product stance: **phone-first full agent** ([PRODUCT_POSITIONING.md](./PRODUCT_POSITIONING.md)); **PC Host is optional** for large repos / desktop-only toolchains. Current factual status is tracked in [CURRENT_STATE.md](./CURRENT_STATE.md) and [DEVICE_E2E_RESULTS.md](./DEVICE_E2E_RESULTS.md).

## Summary

| Expectation | Supported? |
|---|---|
| Full coding agent on phone | **Implemented through Termux**; E2E happy path verified on device when Termux is configured |
| Coding agent in sandbox only | **Partial** — chat/files/patch/ZIP/snapshots; no normal shell executor |
| PC workstation backend | **Implemented when paired**; manual LAN URL works, mDNS depends on same subnet/firewall |
| Phone-only without Termux | **Lite mode** — not equivalent to desktop TUI |
| Control entire phone | **Partial** — picker/share/settings/launch hooks; not full UI automation |
| Control entire PC | **Partial** — paired workspace and PC Host protocol, not arbitrary remote desktop |
| Same as Cursor desktop | **No** — mobile cockpit + Termux/PC executor model |

## By channel

### Phone sandbox: LocalAndroid workspace

| Capability | Status |
|---|---|
| Chat + streaming | Implemented |
| Read/write/patch files in app workspace | Implemented |
| Snapshots | Implemented |
| Project ZIP export/share | Headless E2E PASS |
| Project ZIP import | Headless E2E PASS; system picker UI still manual |
| `exec_shell` | Not supported in sandbox; use Termux or PC Host |
| Git/build/test | Only if tools exist in the active executor; not assumed in sandbox |

### Termux workspace: primary full-agent phone path

| Capability | Status |
|---|---|
| Save/activate Termux workspace path | Implemented |
| Queue approved `exec_shell` through native bridge | Implemented |
| Termux `RUN_COMMAND` intent and result parser | Implemented |
| Continue model turn from stdout/stderr/exit code | Implemented in Rust/mobile path |
| Real hardware happy-path verification | **Verified**: chat PONG, Termux pwd, file create/edit/copy/delete probes on `RFCNC0PWD4E` when Termux configured |
| Requires Termux permission and `allow-external-apps=true` | User setup |

### PC Host workspace: optional large-repo path

| Capability | Status |
|---|---|
| Files, shell, shell stream | Implemented when host is running |
| Git panel + engine git tools | Implemented |
| Terminal / tasks / SSE | Implemented |
| Diagnostics | Rust/TypeScript/Python implemented |
| mDNS discovery | Implemented; blocked by subnet/firewall if phone and PC are not on same LAN |
| Manual LAN URL | Implemented fallback |
| Requires pairing + `deepseek-pc-host` | Optional user setup |
| Release packaging/autostart | Scripts exist; polished release bundle still pending |

### Always available from the phone UI

| Capability | Status |
|---|---|
| Onboarding/settings | Implemented |
| Approvals | Implemented |
| GitHub tools | Implemented; token from settings/environment |
| Web tools | Implemented |
| MCP registry/UI/proxy surfaces | Implemented partially |
| MCP demo echo | E2E PASS |
| External MCP execution | Pending hardening |
| Plan mode | Implemented; tools are not executed |
| In-app update | Implemented for GitHub Releases APK assets; not used for normal source/dev updates |

## Android status

| Item | Status |
|---|---|
| Debug APK build | Verified with `dx build --android --package deepseek-mobile --device RFCNC0PWD4E` |
| Install/launch on physical phone | Verified |
| Android UI render | Verified: onboarding/cockpit render path works |
| Android icon/favicon | Implemented |
| Crash buffer after launch | Empty in latest smoke test |
| Full native flow verification | Mostly automated; picker UI / all-panel manual sweep still pending |

## User-facing interpretation

- **Full agent ready on phone** means: API key configured + valid Termux workspace + Termux permission/setup verified.
- **PC boost ready** means: PC Host running, reachable from the phone, and workspace route active.
- **Sandbox ready** means: chat/files/ZIP/snapshots are available, but shell/build/test are not promised.
