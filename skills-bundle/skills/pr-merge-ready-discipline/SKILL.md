---
name: pr-merge-ready-discipline
description: Keep branches merge-ready—triage CI, resolve conflicts, fix blockers in small commits.
---

## Purpose

Ship changes that are safe to merge: green checks, no conflict markers, clear test evidence.

## Workflow

1. `git status` / `git diff` — know what will land.
2. Run the smallest test set that covers the change (unit, `cargo check`, device E2E if Android).
3. Fix CI failures in focused commits; do not mix unrelated refactors.
4. Resolve merge conflicts by reading both sides; never leave `<<<<<<<` in tree.
5. Summarize: what changed, how verified, known risks.

## On phone (Termux)

- Use `git` in the Termux workspace for project repos.
- Prefer `exec_shell` with captured output before claiming PASS.

## Anti-patterns

- Force-push to `main` without explicit user request.
- Amending pushed commits unless hooks require it and user agreed.
