---
name: playwright-e2e-discipline
description: Browser E2E via playwright-cli on PC; use snapshots before repeated clicks.
---

## Purpose

Automate web UI flows for projects that ship a web front-end (usually tested from PC, not inside the Android app).

## Workflow

1. Navigate → snapshot → interact by ref from snapshot.
2. Max ~4 blind retries; change strategy after new evidence.
3. Screenshot for visual assertions when DOM is insufficient.

## Relation to DeepSeek-Mobile

- Android app UI is tested via **adb** / device E2E scripts, not Playwright.
- Use Playwright when the **user's project** is a website and tests run on PC or Termux with browser tooling.

## Anti-patterns

- Clicking without a fresh snapshot after navigation.
- Using browser automation to bypass missing Android MCP tools.
