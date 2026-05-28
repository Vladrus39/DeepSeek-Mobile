---
name: monitored-loops-discipline
description: Poll logs/probes with timeouts; stop after N failures; never infinite wait loops.
---

## Purpose

Safe polling for E2E probes, Termux callbacks, MCP HTTP, ZIP transfer flags.

## Pattern

1. Set a **deadline** (e.g. 45–120s).
2. Poll every 2–4s a result file or `logcat -d` slice.
3. On PASS/FAIL line, exit immediately.
4. On timeout: dump last 30–50 relevant log lines and mark FAIL.

## Probe files (app data dir)

| Flag | Result |
|------|--------|
| `.agent_turn_probe_requested` | `.agent_turn_probe_result` |
| `.zip_transfer_probe_requested` | `.zip_transfer_probe_result` |
| `.zip_transfer_probe_import_requested` | same result file |

## Anti-patterns

- Sleeping 60s without checking intermediate state.
- Claiming success because the app “should” have finished.
