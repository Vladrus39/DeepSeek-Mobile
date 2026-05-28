---
name: datadog-mcp-setup
description: First-time Datadog MCP plugin setup—auth, domain, verify tools respond.
---

## Purpose

Connect Datadog via MCP when the user debugs production (logs, metrics, traces).

## Prerequisites

- Datadog MCP server enabled in Cursor/plugin (`plugin-datadog-datadog`).
- User org access and API/app keys per Datadog docs.

## Setup steps

1. Run `mcp_auth` for the Datadog server if tools return 401.
2. Configure site/domain (US/EU) per org.
3. Call a read-only tool (e.g. list monitors or query logs) to verify.

## Mobile

- Register HTTP MCP in `mcp.json` only if Datadog exposes a reachable endpoint for your network.
- Most Datadog MCP usage remains **PC Cursor**; phone agent can consume pre-synced summaries.
