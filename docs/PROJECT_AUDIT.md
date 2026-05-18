# DeepSeek-Mobile project audit

**Audit refreshed:** 2026-05-18  
**Reference project:** `Hmbown/DeepSeek-TUI`

## Executive summary

DeepSeek-Mobile has moved beyond a prototype. The project now has a real mobile-first runtime, a functioning PC gateway, approvals, snapshots, diagnostics, persisted settings, and a mostly complete tool layer.

The main remaining work is no longer “port the basics from the TUI.” It is now about **closing integration gaps**:

- make native Android and Termux execution fully real;
- connect already-written subsystems to the main lifecycle;
- replace a few visual placeholders with real runtime-backed behavior;
- add durable orchestration features that still do not exist.

## What is solidly implemented

### Runtime and safety

- DeepSeek streaming with reasoning deltas
- Session/runtime persistence
- Approval storage and continuation
- Workspace boundaries and tool capability gating
- Saved mobile settings applied to real turns and approval continuations
- GitHub token propagation from saved settings into the tool context

### Tools and editing

- File read/write/edit/delete/copy/move/list
- `apply_patch`
- Shell, git, web, GitHub tools
- Snapshot create/list/restore
- Pre-tool and post-turn snapshot hooks

### PC execution path

- Authenticated PC-host gateway
- Endpoint candidate planning, health scoring, and failover
- Pairing ZIP generation
- mDNS discovery
- Command streaming
- Git operations
- Terminal sessions
- Host logs and health
- Rust / TypeScript / Python diagnostics

### Mobile surfaces

- Chat timeline and approval cards
- Onboarding/settings
- Files, snapshots, diagnostics, terminal, PC host, and Git panels
- Native bridge contracts for document picker, discovery, terminal, and sharing

## Important improvements completed in the latest tranche

1. **Persisted settings now matter at runtime.**  
   Before this, UI settings were saved but normal turns and approval continuations still rebuilt the engine with defaults.

2. **Pairing now activates a real workspace.**  
   Online discovery promotes an active route, the pairing screen can build a `WorkspaceConnection`, and “Open PC workspace” persists it. Future turns reload that connection through `MobileRuntimeConfig`.

3. **Diagnostics reporting is more truthful.**  
   Multi-provider local diagnostics no longer collapse unrelated states into a misleading empty/unavailable result.

4. **Pairing no longer defaults to an empty auth token.**  
   New pairing requests use a generated token.

## Partial implementations that still need completion

| Area | Current reality | What remains |
|---|---|---|
| Git UI | Surface exists | Wire buttons to real status/diff/branch/commit/push/pull actions |
| Files diff UI | Tree and file preview are real | Replace illustrative diff preview with actual patch/project diff data |
| Terminal | UI + host sessions exist | Persist sessions and finish Android runtime wiring |
| Termux | Request/result contract scaffolding exists | Implement the native bridge and executor lifecycle |
| Android bridge | Kotlin/Rust contracts exist | Final host integration and manual verification |
| Diagnostics | Hooks and providers exist | Inject results into the next model turn |
| Snapshots | Local path is integrated | Add PC-gateway snapshot path |
| Model routing | `ModelRouter` exists | Use it in actual turn orchestration |
| Context management | `ContextManager` exists | Use it in actual prompt assembly |
| Auto-commit | Helper exists | Invoke it from successful-turn lifecycle when enabled |
| Background tasks | Some host task detection exists | Add durable task records, queue, UI, and artifacts |
| Extensibility | Not yet present | Add runtime API, MCP, plugins, skills |

## Comparison with DeepSeek-TUI

| Area | Mobile status |
|---|---|
| Streaming | Done |
| Approval workflow | Done |
| File editing | Done |
| Patch application | Done |
| Shell execution | Done through PC-host; Termux still incomplete |
| Git tooling | Core done; mobile workflow partial |
| Web/GitHub tools | Done in core |
| Diagnostics | Providers done; model reinjection still pending |
| Snapshots | Local done; remote pending |
| Runtime API | Missing |
| Durable tasks | Missing |
| MCP/skills/plugins | Missing |
| Android-native execution | Partial |

## Recommended next execution order

1. Finish Android host integration and Termux bridge.
2. Wire Git UI actions and engine auto-commit.
3. Feed diagnostics into the next turn and replace fake diff preview with real data.
4. Add remote snapshots and terminal persistence.
5. Add durable tasks, runtime API, then extensibility.

## Audit conclusion

The project is now in a good place architecturally. The risk is no longer lack of capability; it is **capability drift** — features that exist in code but are not yet fully connected end-to-end. The right strategy is to keep finishing vertical slices, not start broad new subsystems before the current ones are truly closed.
