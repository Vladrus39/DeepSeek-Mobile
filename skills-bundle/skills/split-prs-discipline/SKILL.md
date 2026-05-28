---
name: split-prs-discipline
description: Split large changes into small reviewable PRs with clear scope per branch.
---

## Purpose

Keep reviews mergeable: one concern per PR, minimal cross-file churn.

## Split criteria

- **Independent features** → separate branches.
- **Refactor + feature** → refactor PR first, then feature.
- **Android + Rust core** → split only when builds/tests are separable.

## Workflow

1. Identify logical chunks from `git diff`.
2. Create branch per chunk; cherry-pick or restage files.
3. Each PR: summary bullets + test plan checklist.
4. Push with `gh pr create` on PC when user asks.

## Phone note

Use Termux `git` for local commits; opening PRs usually needs PC + `gh` auth.
