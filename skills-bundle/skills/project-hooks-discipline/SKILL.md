---
name: project-hooks-discipline
description: Automate repeatable agent workflows with hooks/scripts; keep them idempotent and logged.
---

## Purpose

Mirror Cursor “hooks” discipline for DeepSeek-Mobile: scripts that run on events (build, E2E, push skills).

## Repo hooks (examples)

- `scripts/push-skills-to-device.ps1` — install skills bundle.
- `scripts/device-e2e-*.ps1` — device verification probes.
- `full-rebuild-verify.ps1` — local rebuild smoke.

## Rules

- Scripts must be non-interactive and exit non-zero on failure.
- Log artifacts under `target/` with timestamps.
- Never skip git hooks (`--no-verify`) unless the user explicitly requests it.

## Adding a new hook

1. Place under `scripts/` with a descriptive name.
2. Document in `docs/DEVICE_E2E_RESULTS.md` or README.
3. Run once on device/PC and record PASS/FAIL.
