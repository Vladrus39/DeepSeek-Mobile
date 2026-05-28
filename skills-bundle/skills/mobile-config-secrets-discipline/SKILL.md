---
name: mobile-config-secrets-discipline
description: Manage config.json, secrets.enc, API keys—never commit or log secrets.
---

## Purpose

Safe handling of mobile runtime config on device.

## Files (app data dir)

- `config.json` — model, modes, workspace pointers.
- `secrets.enc` — encrypted API keys.
- `mcp.json` — MCP server list (no secrets in URLs unless required).

## Rules

- Never paste API keys into chat, commits, or E2E logs.
- Use adb `run-as` only on debug builds; warn on release.
- After config edits: restart agent turn or reopen relevant tab (MCP, Skills).

## Verification

```powershell
adb shell run-as com.deepseek.mobile cat files/deepseek-mobile/config.json
# Redact secrets in reports
```
