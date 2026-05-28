---
name: analytical-artifacts-discipline
description: Produce structured reports (tables, timelines, checklists) when analysis is the deliverable.
---

## Purpose

When the user needs investigation output (not just code), deliver scannable structure.

## When to use

- E2E result summaries, architecture reviews, billing/debug investigations.
- Comparing options with trade-offs.

## Format

- Lead with **conclusion** in 1–2 sentences.
- Use headings, bullet lists, and tables for metrics.
- Cite file paths and command output snippets as evidence.
- End with **next actions** (ordered, testable).

## Mobile constraints

- Avoid huge paste walls; link to `files/deepseek-mobile/` artifacts or adb-captured logs.
- For visual-heavy data on PC, note that full canvas/UI may be desktop-only; still structure markdown clearly on phone.
