---
name: datadog-mcp-config
description: Tune Datadog MCP domain/org; troubleshoot stale or wrong-site connections.
---

## Purpose

Fix Datadog MCP misconfiguration (wrong site, org switch, expired auth).

## Checks

- Confirm Datadog site URL matches org (`datadoghq.com` vs `datadoghq.eu`).
- Re-auth if queries return 403/401.
- Disable unused toolsets to reduce noise (see datadog-mcp-toolsets skill).

## Output discipline

- Paste query time range and service/env filters used.
- Link monitor/dashboard IDs when citing incidents.
