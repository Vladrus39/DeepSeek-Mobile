---
name: termux-cli-status-discipline
description: Termux shell hygiene—pwd, env, node/rust paths, compact status before long commands.
---

## Purpose

Avoid wrong-directory and wrong-tool failures in Termux workspace.

## Before heavy work

```sh
pwd
ls -la
which rustc cargo node python3 git 2>/dev/null
```

## Status discipline

- Print **cwd** when switching projects.
- After install: verify binary with `--version`.
- Long builds: use `timeout` or background + log file when appropriate.

## DeepSeek-Mobile

- Active workspace path is in app settings / `termux_workspace.json`.
- All `exec_shell` runs should assume that cwd unless user specifies otherwise.
