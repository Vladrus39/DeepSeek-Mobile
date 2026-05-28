---
name: git-workflow-discipline
description: Safe Git workflow for DeepSeek-Mobile: status/diff/log, reversible changes, clean commits.
---

## Purpose

Keep repo history clean and changes reviewable. Never lose work, never rewrite shared history, and always attach evidence (diffs) when describing changes.

## Mandatory pre-flight (every time)

Before you claim “X is changed” or “I fixed Y”, capture:

```bash
git status
git diff
git log -5 --oneline
```

If there are untracked scripts/logs/build artifacts, decide deliberately whether they belong in the repo.

## Branch discipline

- Prefer working on a branch, but if the task explicitly requires `main`, record that decision in the commit message context.
- Do not assume branch is up to date—verify:

```bash
git status -sb
git remote -v
```

## Commit message style (repo-friendly)

Use:

- **Short imperative summary** (what/why)
- Optional blank line
- **Details**: constraints, evidence, follow-ups

Example:

```
skills: add DeepSeek-Mobile practical bundle

Add six device-focused skills plus install docs and a push script to seed /sdcard and optionally run-as internal files.
```

## No-force policy

Never use:

- `git push --force`
- `git reset --hard` (unless you can prove nothing valuable is lost)
- `git rebase -i` in non-interactive environments

If you must rewrite local history, ensure it has not been pushed and document the reason.

## Staging discipline

Stage intentionally:

```bash
git add -p
```

If interactive staging is not possible, stage by path:

```bash
git add skills-bundle/ scripts/push-skills-to-device.ps1
```

Before committing:

```bash
git diff --staged
```

## Verification before pushing

At minimum:

- `git status` is clean (or only expected leftovers)
- `git diff --staged` matches the intended change
- You can describe exactly how to verify (commands or scripts)

Then push safely:

```bash
git push origin HEAD
```

If `main` is protected and push fails, stop and report the exact server error.

