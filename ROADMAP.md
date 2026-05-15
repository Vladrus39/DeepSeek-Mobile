# DeepSeek-Mobile Roadmap

## Vision

DeepSeek-Mobile is an Android-first coding agent. It should preserve the power of DeepSeek-TUI while providing a mobile workflow for real projects.

It must support three execution modes:

1. **Local Android workspace** — safe file operations and patching inside app-managed storage.
2. **Termux bridge** — execute development commands on the same Android device through Termux.
3. **Remote executor** — run heavy tasks on Y-lit, a VM, or another backend while the phone stays the control panel.

## Phase 0 — Stabilize repository

- [x] Add missing session module.
- [ ] Add CI for `cargo check`.
- [ ] Ensure workspace compiles.
- [ ] Add basic architecture documents.
- [ ] Add clear MVP status tracking.

## Phase 1 — Real chat core

- [ ] Store API key through a mobile-safe settings flow.
- [ ] Use full message history, not only the last user message.
- [ ] Add request/response error model.
- [ ] Add provider abstraction for DeepSeek and OpenAI-compatible APIs.
- [ ] Add non-streaming chat tests.

## Phase 2 — Streaming agent events

- [ ] Add `AgentEvent` enum.
- [ ] Add streaming API client.
- [ ] Render text deltas in the mobile UI.
- [ ] Render reasoning/status events separately from final text.
- [ ] Persist event timeline for resume.

## Phase 3 — Workspace and files

- [ ] Add workspace model.
- [ ] Add project import/export as ZIP.
- [ ] Add file tree.
- [ ] Implement `read_file`.
- [ ] Implement `write_file`.
- [ ] Implement `edit_file` / `apply_patch`.
- [ ] Add diff viewer.
- [ ] Add patch approval screen.

## Phase 4 — Tool-calling loop

- [ ] Define tool schemas.
- [ ] Send tool specs to model.
- [ ] Parse tool calls.
- [ ] Execute tools through approval policy.
- [ ] Return tool results to model.
- [ ] Stop only on final answer.

## Phase 5 — Execution policy

- [ ] Add Plan mode.
- [ ] Add Agent mode.
- [ ] Add YOLO mode.
- [ ] Add dangerous command blocker.
- [ ] Add workspace boundary checks.
- [ ] Add per-tool approval rules.

## Phase 6 — Termux and remote execution

- [ ] Add executor trait.
- [ ] Add local Android executor.
- [ ] Add Termux bridge executor.
- [ ] Add remote Y-lit executor.
- [ ] Stream command output to UI.
- [ ] Persist command logs.

## Phase 7 — Large project support

- [ ] Add project index.
- [ ] Add file summaries.
- [ ] Add symbol search hooks.
- [ ] Add test/build diagnostics.
- [ ] Add snapshot/rollback.
- [ ] Add cost/context tracking.

## Phase 8 — Plugins and integrations

- [ ] Add MCP-compatible plugin host.
- [ ] Add GitHub tools.
- [ ] Add Y-lit deploy tools.
- [ ] Add task queue.
- [ ] Add background jobs.
- [ ] Add LSP diagnostics through remote or Termux executor.
