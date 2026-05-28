---
name: cursor-sdk-automation
description: Use Cursor SDK/agents from PC scripts when automating outside the phone app.
---

## Purpose

Phone agent is primary; **Cursor SDK** (`@cursor/sdk`, `cursor-sdk`) is for PC-side automation (CI, bots).

## When to use SDK vs phone

| Task | Surface |
|------|---------|
| On-device coding, Termux, MCP from phone | DeepSeek-Mobile app |
| GitHub Actions, backend bots, batch refactors | Cursor SDK on PC |

## Rules

- Read current SDK skill/docs before API calls; versions change.
- Do not embed API keys in repos; use env/secrets.
- Phone E2E still required for Android-specific flows.

## Reference

Cursor SDK skill on desktop: install package, `Agent.create`, streaming, cancellation handling.
