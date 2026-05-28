---
name: datadog-mcp-toolsets
description: Enable/disable Datadog MCP toolsets to match the investigation scope.
---

## Purpose

Avoid tool overload: enable only toolsets needed for logs, metrics, traces, or incidents.

## Practice

- Start minimal (logs OR metrics), expand if blocked.
- Document which toolset answered the question.
- On phone, prefer summarized findings over raw huge JSON in chat.

## When not available

If Datadog MCP tools are not in the session, state that clearly and use adb/logcat or project logs instead.
