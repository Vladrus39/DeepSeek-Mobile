# Tool audit — DeepSeek-Mobile

**Updated:** 2026-05-29  
**Device:** `RFCNC0PWD4E`  
**Registry:** `default_mobile_tool_registry()` + dynamic `mcp__<server>__<tool>` proxies from `mcp.json`

## Built-in tools (26)

| Tool | Category | On-device E2E | Notes |
|------|----------|---------------|-------|
| `read_file` | File | **PASS** | `device-e2e-file-create` (via agent + Termux) |
| `write_file` | File | **PASS** | `device-e2e-file-create` |
| `list_dir` | File | **PASS** | `device-e2e-project-workflow` (`ls`) |
| `edit_file` | File | **PASS** | `device-e2e-project-workflow` (hello.txt edit) |
| `delete_file` | File | **PASS** | `device-e2e-delete-file.ps1` (Termux `rm`, 2026-05-29) |
| `copy_file` | File | **PASS** (Termux) | `device-e2e-copy-file.ps1` (2026-05-29) |
| `move_file` | File | Not probed | Same |
| `read_many_files` | File | Not probed | Same |
| `file_ops` | File | Not probed | Composite helper |
| `apply_patch` | File | **PASS** (sandbox) | `device-e2e-tools-sandbox.ps1` on `files/.../workspace` (2026-05-29) |
| `exec_shell` | Shell | **PASS** | `cat`, `ls`, `git status` in E2E scripts |
| `git` | Git | **PASS** (partial) | `git status` in project-workflow |
| `workspace_overview` | Workspace | **PASS** (sandbox) | `device-e2e-tools-sandbox.ps1` |
| `file_summary` | Workspace | Not probed | |
| `snapshot_create` | Snapshot | Not probed | |
| `snapshot_list` | Snapshot | Not probed | |
| `snapshot_restore` | Snapshot | Not probed | |
| `phone_control` | Phone | Not probed | Native bridge actions |
| `open_path` | Phone | Not probed | |
| `web_fetch` | Network | BLOCKED | Needs token/approval; no stable LAN probe in CI |
| `web_search` | Network | BLOCKED | Same |
| `github_repo` | GitHub | BLOCKED-needs-token | `config.json` has `github_repo: null` on device |
| `github_pr` | GitHub | BLOCKED-needs-token | Same |
| `github_issue` | GitHub | BLOCKED-needs-token | Same |
| `github_browse` | GitHub | BLOCKED-needs-token | Same |
| `github_push_file` | GitHub | BLOCKED-needs-token | Same |

## MCP proxy tools

| Pattern | E2E | Notes |
|---------|-----|-------|
| `mcp__demo__echo` | **PASS** | `scripts/device-e2e-mcp.ps1` + LAN `mcp_demo_http_server.py` |
| Other servers | Config-dependent | Add entries to `files/deepseek-mobile/mcp.json`; refresh MCP tab |

## Transfer / UI (not ToolRegistry)

| Flow | E2E | Script |
|------|-----|--------|
| ZIP export + share | **PASS** | `device-e2e-zip-export.ps1` |
| ZIP import (headless) | **PASS** | `device-e2e-zip-import.ps1` |
| ZIP import (system picker) | Manual | Steps in `docs/ZIP_IMPORT_UI_TEST.md`; MIME fix for `octet-stream` |

## Skills (21)

Bundled under `skills-bundle/skills/`. Enabled skills inject **full SKILL.md** into the agent system prompt. Install: `scripts/push-skills-to-device.ps1`.

## Recommended next probes

1. `file_summary`, snapshots, `phone_control` / `open_path`  
2. GitHub tools after adding PAT to device `secrets.enc` / config  
3. `web_fetch` to a stable LAN URL with approval disabled in probe  
4. PC Host pairing (last phase) — `device-full-verify.ps1` optional `-SkipPcHost`  
