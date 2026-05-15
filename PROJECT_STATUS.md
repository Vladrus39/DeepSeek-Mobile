# DeepSeek-Mobile — Project Status

## Current goal

Build a real Android-first DeepSeek coding agent based on the DeepSeek-TUI architecture, without reducing it to a simple chat client.

The mobile app must keep the important capabilities of the original TUI agent:

- agent loop;
- session and turn state;
- tool execution;
- approval policy;
- file operations;
- shell / Termux / remote executor support;
- git operations;
- project snapshots and rollback;
- plugin / MCP-ready architecture.

## Current implementation state

### Implemented

- Rust workspace with separate `core` and `mobile` crates.
- Dioxus mobile UI shell.
- Basic chat screen.
- DeepSeek API client.
- Basic `Config` model.
- Basic `DeepSeekAgent` wrapper.
- Basic tool registry.
- Stub tools for file operations, shell and git.
- Basic `Session` model.

### Not implemented yet

- Real streaming response handling.
- Real reasoning block rendering.
- Full message-history handling.
- Real tool-calling loop.
- Real file operations.
- Real shell execution.
- Termux bridge.
- Remote Y-lit executor.
- Workspace explorer.
- Patch/diff approval screen.
- Snapshots and rollback.
- SQLite or durable session persistence.
- MCP/plugin host.
- LSP diagnostics.
- CI-backed Android build.

## Important architectural decision

DeepSeek-Mobile should not be a direct copy of the terminal UI.

The correct target is:

```text
Android UI + mobile UX
        ↓
Reusable Rust agent core
        ↓
Local / Termux / Remote executors
        ↓
DeepSeek API + future compatible providers
```

This keeps the project suitable for large codebases while still making it usable from a phone.

## Immediate priorities

1. Keep `main` buildable with `cargo check`.
2. Add CI for workspace checks.
3. Replace placeholder tools with real file operations.
4. Add streaming events from the API client.
5. Add proper session/history handling.
6. Add approval-aware agent loop.
7. Add project workspace model.
8. Add Termux/remote executor abstraction.
