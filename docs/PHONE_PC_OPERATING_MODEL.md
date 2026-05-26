# Phone and PC operating model

**Updated:** 2026-05-26

This document defines how DeepSeek-Mobile should work across Android, Termux, a paired PC, and optional DeepSeek-TUI compatibility. It is the product architecture target, not only an implementation note.

## Core principle

The Android app is the primary user interface and orchestration layer. See **PRODUCT_POSITIONING.md** — phone-first full agent; PC optional.

Execution backends (priority):

1. **Phone + Termux workspace** — **default full agent path**: shell, git, build, test in a Termux project directory (TUI parity on device).
2. **Phone app local workspace** — lite mode: app-private files, attachments, snapshots; no normal shell.
3. **Phone + PC Host workspace (optional)** — when the project is too large for the phone or the user prefers desktop toolchains; phone UI, PC execution.
4. **Remote runtime later** — optional, same pattern as PC Host.

The PC is **not** required to launch, chat, or run a full agent. Termux + toolchain setup is the phone-native equivalent of a desktop coding terminal. PC Host is for **scaling up**, not for “being a real product.”

## Capability matrix

| Mode | What works | Main limits | Best use |
|---|---|---|---|
| Android app only | Chat, attachments, app-private workspace files, approvals, snapshots, patching, model planning | No normal local shell executor; git/build/test depends on binaries being available in the app environment, which should not be assumed | Review, planning, small file edits, attachment-based work |
| Android + Termux | **Full agent on phone** — same tool surface as TUI with Termux as executor | Termux install, `termux.properties`, RUN_COMMAND, valid path in Settings | **Main recommended workflow** |
| Android + PC Host | Optional workstation boost from phone UI when project exceeds phone capacity | Paired PC host on LAN/tunnel | Optional — not default positioning |
| Android + existing DeepSeek-TUI | Optional config/skills compatibility only | Do not drive the TUI UI or depend on its internals | Migration/interoperability, not core runtime |

## Recommended product stance

DeepSeek-Mobile should not depend on DeepSeek-TUI being installed on the PC.

Reasoning:

- A TUI is an interactive terminal UI, not a stable machine protocol.
- Driving another UI makes automation brittle across TUI versions.
- The mobile app already has its own approvals, timeline, settings, snapshots and routing; duplicating that through the TUI would create two sources of truth.
- A small versioned PC Host protocol is easier to secure, test and keep backward-compatible.

The right model is:

```text
Android app
  -> versioned DeepSeek PC Host protocol
  -> deepseek-pc-host binary/service on PC
  -> project filesystem, git, shell, diagnostics, terminal, tasks
```

DeepSeek-TUI compatibility should be optional and data-oriented:

```text
Existing DeepSeek-TUI installation
  -> optional config import
  -> optional skills/prompts discovery
  -> optional shared conventions
  -> no dependency on TUI UI internals
```

## Pairing and PC bootstrap flow

Current implemented flow:

1. Android creates a PC pairing bundle/ZIP.
2. The bundle contains `pairing.json`, environment file and launch scripts.
3. User opens the launcher on the PC.
4. `deepseek-pc-host` starts with the workspace root, token and bind address.
5. Android discovers/probes the host and persists the selected PC workspace connection.
6. Normal engine turns use that active PC workspace automatically.

Current bootstrap behavior:

1. If the pairing bundle contains a matching `deepseek-pc-host` binary next to the launcher or under `bin/`, run that local binary.
2. Else if `deepseek-pc-host` is on `PATH`, run it.
3. Else show a clear install instruction.

Target v1 packaging should add the missing distribution piece:

1. Include the matching `deepseek-pc-host` binary in the PC pairing/release package when possible.
2. Offer a signed installer/download step only after explicit user confirmation.
3. Optionally offer “install as service/autostart” after the first successful manual run.

This keeps pairing simple while avoiding a hidden dependency on DeepSeek-TUI.

## What “sync with PC” means

The pairing file is not project synchronization in the Dropbox/Git sense. It is a secure capability grant:

- which PC host to trust;
- which workspace root is granted;
- which token the phone must use;
- which endpoint(s) to try.

When a PC workspace is active, the PC project remains the source of truth. The phone reads/edits/runs commands through the PC Host. The phone does not need to copy the whole project locally.

Separate import/export features can still exist for phone-only workflows, but they are not the same as PC pairing.

## Phone-only coding target

The phone app should be useful even with no PC:

- chat and planning;
- file attachment ingestion;
- app-private workspace browsing/editing;
- approval flow;
- snapshots/rollback;
- patch application;
- saved sessions/settings;
- Termux route when installed.

But “full coder like on PC” requires an executor with real developer tools. On Android that means Termux or a bundled/sandboxed runtime. For v1, Termux is the pragmatic local execution backend.

## PC coding target

With PC Host paired, the phone should feel like a full coding agent controlling the PC:

- browse/read/edit project files;
- apply patches;
- run tests/builds/linters;
- see diagnostics;
- use git status/diff/commit/push/pull;
- run terminal commands;
- start/stop/list background tasks;
- reconcile active PC-running tasks in the mobile Tasks panel;
- snapshot/restore workspace state;
- recover from reconnect/failover.

This is the main power-user mode.

## DeepSeek-TUI relationship

Recommended relationship: **compatible, not dependent**.

Good integrations:

- import/read compatible skills from known directories;
- reuse prompt/tool conventions where stable;
- optionally detect a TUI install and offer to import settings;
- document migration from TUI to Mobile + PC Host.

Avoid:

- launching the TUI and scraping its terminal UI;
- requiring a specific TUI version on the PC;
- making PC Host a plugin that only works inside TUI;
- depending on private TUI internals for file/shell/git execution.

If a future TUI exposes a stable headless protocol, DeepSeek-Mobile can add it as another backend. Until then, `deepseek-pc-host` remains the correct integration point.

## Security model

- Pairing token is generated by the phone and stored in the pairing bundle.
- PC Host serves only the granted workspace root.
- Path traversal is rejected by the host/core boundary.
- Destructive operations require approval unless the user explicitly chooses a safe auto-approved mode.
- LAN/direct routes are preferred before tunnel/internet routes.
- Public remote access must require explicit configuration and stronger transport guarantees.

## Current implementation status

Verified in this checkpoint:

- Dioxus Android debug APK builds through the repo-local Android environment.
- APK installs and launches on a physical phone.
- First setup screen renders.
- Android icon resources are packaged.
- Dioxus native library loading, JNI package alignment and startup manifest crash are fixed.

Implemented:

- PC pairing bundle generation.
- PC Host HTTP/SSE server.
- Auth token support.
- mDNS discovery and endpoint failover.
- Active PC workspace persistence.
- PC file/git/shell/diagnostics/snapshot/task routing.
- Mobile files/git/tasks/MCP/skills panels.
- Termux request queue and model continuation after callback.
- Termux workspace selector in Settings with persistent runtime activation.
- Durable task artifacts/log capture and PC-host runtime task HTTP endpoints.
- Core ZIP workspace import/export helpers.
- Files panel project import/export UI for local phone workspace archives.
- Mobile Tasks panel sync/stop controls for active PC-host running tasks.
- Dioxus Android host: JNI, `android_host` callbacks, `MainActivity`, Kotlin coordinator and bridge packaging.
- Health/setup/chat quick-action surfaces.
- Plan mode skips tool execution; LocalAndroid `exec_shell` returns actionable errors.

Still needed:

- hardware verification of Android picker chat attachments;
- hardware verification of Files Import ZIP;
- hardware verification of Files Export ZIP/native share;
- hardware verification of Termux `RUN_COMMAND` permission/result callback;
- LAN verification of PC Host discovery/persisted route;
- PC Host release package that bundles `deepseek-pc-host` for Windows/macOS/Linux;
- optional PC service/autostart installer;
- signed Android APK/AAB.

## Recommended next implementation order

1. Run the native Android hardware checklist on the connected phone.
2. Fix any picker/import/export/share/Termux/discovery issues found during that checklist.
3. Add signed Android release packaging.
4. Package PC Host binaries and optional service/autostart installer.
5. Finish MCP stdio session reuse and controlled external MCP execution.
