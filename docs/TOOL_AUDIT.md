# Tool audit — DeepSeek-Mobile

**Updated:** 2026-05-28  
**Device:** `RFCNC0PWD4E`  
**Registry:** `default_mobile_tool_registry()` + dynamic `mcp__<server>__<tool>` proxies from `mcp.json`

## Built-in tools (26)

| Tool | Category | On-device E2E | Notes |
|------|----------|---------------|-------|
| `read_file` | File | **PASS** | `device-e2e-file-create` (via agent + Termux) |
| `write_file` | File | **PASS** | `device-e2e-file-create` |
| `list_dir` | File | **PASS** | `device-e2e-project-workflow` (`ls`) |
| `edit_file` | File | **PASS** | `device-e2e-project-workflow` (hello.txt edit) |
| `delete_file` | File | Not probed | Routed via Termux when workspace is Termux |
| `copy_file` | File | Not probed | Same |
| `move_file` | File | Not probed | Same |
| `read_many_files` | File | Not probed | Same |
| `file_ops` | File | Not probed | Composite helper |
| `apply_patch` | File | Not probed | Needs dedicated patch E2E |
| `exec_shell` | Shell | **PASS** | `cat`, `ls`, `git status` in E2E scripts |
| `git` | Git | **PASS** (partial) | `git status` in project-workflow |
| `workspace_overview` | Workspace | Not probed | Local/Termux read |
| `file_summary` | Workspace | Not probed | |
| `snapshot_create` | Snapshot | Not probed | |
| `snapshot_list` | Snapshot | Not probed | |
| `snapshot_restore` | Snapshot | Not probed | |
| `phone_control` | Phone | Not probed | Native bridge actions |
| `open_path` | Phone | Not probed | |
| `web_fetch` | Network | Not probed | Requires approval + network |
| `web_search` | Network | Not probed | |
| `github_repo` | GitHub | Not probed | Needs token |
| `github_pr` | GitHub | Not probed | |
| `github_issue` | GitHub | Not probed | |
| `github_browse` | GitHub | Not probed | |
| `github_push_file` | GitHub | Not probed | |

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
| ZIP import (system picker) | Manual | User selects `.zip` in UI |

## Skills (21)

Bundled under `skills-bundle/skills/`. Enabled skills inject **full SKILL.md** into the agent system prompt. Install: `scripts/push-skills-to-device.ps1`.

## Recommended next probes

1. `apply_patch` on a small file in Termux workspace  
2. `workspace_overview` after seeding a mini-repo  
3. GitHub tools with a test PAT (PC or Termux)  
4. `web_fetch` to a stable LAN URL  
