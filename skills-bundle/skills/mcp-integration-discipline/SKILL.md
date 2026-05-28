---
name: mcp-integration-discipline
description: Configure MCP servers on phone, call mcp__server__tool with approval, verify HTTP/stdio results.
---

## Purpose

Use MCP tools safely and reproducibly from DeepSeek-Mobile.

## Configuration

- Registry file: `files/deepseek-mobile/mcp.json` (array of server configs).
- Transports:
  - `http_sse` with `url` (POST JSON-RPC to `/mcp`)
  - `stdio` with `command` + `args` (desktop only; not for phone-first workflows)

Example HTTP server entry:

```json
[
  {
    "name": "demo",
    "transport": "http_sse",
    "url": "http://<PC_LAN_IP>:3333",
    "enabled": true,
    "description": "LAN MCP test server"
  }
]
```

Phone and PC must be on the same Wi‑Fi. Prefer PC IP from `ipconfig` / `hostname -I`, not `localhost`.

## Tool naming

Proxy tools appear as:

`mcp__<server_name>__<tool_name>`

Example: `mcp__demo__echo` with args `{"text":"PHONE"}`.

## Execution rules

- MCP tools require **approval** — wait for user or YOLO probe before assuming success.
- Never paraphrase tool output; paste the JSON/text returned by the tool.
- If connection fails, check: URL reachable from phone browser/curl, firewall, `mcp.json` syntax, server `enabled: true`.
- Refresh MCP tab in app after editing `mcp.json`.

## Verification checklist

1. `tools/list` returns tools (MCP tab shows tool count > 0 or declared_tools).
2. Call one tool with minimal args; confirm exact payload in chat/work log.
3. Log PC server access (HTTP 200) when using LAN server.

## Local test server (repo)

```powershell
python .\scripts\mcp_demo_http_server.py --host 0.0.0.0 --port 3333
```

Push config to device, open app → MCP → refresh.
