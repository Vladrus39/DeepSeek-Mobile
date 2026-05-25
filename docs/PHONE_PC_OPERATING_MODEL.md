# Phone and PC operating model

**Updated:** 2026-05-25

This document defines how DeepSeek-Mobile should work across Android, Termux, a paired PC, and optional DeepSeek-TUI compatibility. It is the product architecture target, not only an implementation note.

## Core principle

The Android app is the primary user interface and orchestration layer. Execution backends are selectable per workspace:

1. **Phone app local workspace** — app-private files, attachments, approvals, snapshots and model orchestration.
2. **Phone + Termux workspace** — Android-local command execution through a saved Termux workspace after approval.
3. **Phone + PC Host workspace** — the phone controls the agent while the PC executes file, shell, git, diagnostics, terminal and task operations inside the granted project.
4. **Remote runtime later** — same idea as PC Host, but over a remote runtime when explicitly configured.

The PC is not required for the app to launch or chat. The PC is required for the best “full coding workstation” experience on large real projects unless Termux is configured with the needed toolchains.

## Capability matrix

| Mode | What works | Main limits | Best use |
|---|---|---|---|
| Android app only | Chat, attachments, app-private workspace files, approvals, snapshots, patching, model planning | No normal local shell executor; git/build/test depends on binaries being available in the app environment, which should not be assumed | Review, planning, small file edits, attachment-based work |
| Android + Termux | Local Android coding with a saved Termux workspace, approved shell commands, git/build/test if installed in Termux | User must install/configure Termux packages; Android permissions and external app settings must be correct; final host verification is still pending | Phone-only coding for small/medium repos |
| Android + PC Host | Full workstation execution from phone UI: files, git, tests, builds, diagnostics, terminal, long tasks | Requires paired PC host running and reachable over LAN/direct/tunnel | Main recommended pro workflow |
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

Still needed:

- final Dioxus Android host adapter;
- device/emulator verification of bridge callbacks;
- final Android picker/share device verification for project import/export;
- PC Host release package that includes or installs the host binary;
- optional PC service/autostart installer;
- runtime SSE/live event streaming;
- live PC-running-task synchronization/reconciliation.

## Recommended next implementation order

1. Update Android host integration docs to reflect that Rust/mobile Termux continuation is already done, leaving final host adapter/device verification.
2. Add PC Host bootstrap/release plan: bundled binary first, PATH fallback second, explicit install third.
3. Implement final Dioxus/native host adapter contract.
4. Verify Android project import/export picker/share on device or emulator.
5. Add runtime SSE/live event streaming only after the host/runtime boundaries are stable.
6. Add PC-running-task reconciliation and release packaging.
