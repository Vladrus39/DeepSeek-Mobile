# Device E2E results

**Updated:** 2026-05-29  
**Device:** Samsung SM_G781B · `RFCNC0PWD4E`  
**Package:** `com.deepseek.mobile`

## Automated probes (ADB)

| Check | Status | Notes |
|-------|--------|-------|
| API HTTP (`api_probe`) | PASS (last run) | `http=200` in `.api_probe_result` |
| Agent turn (`agent_turn_probe`) | PASS (last run) | `PROBE_OK` in `.agent_turn_probe_result` |
| Termux calibration | PASS | `adb-control -Action Calibrate` 2026-05-28 |
| Termux pwd agent (`device-termux-pwd-probe`) | PASS | `exit=0`, stdout `.../deepseek-project` — 2026-05-28 ~13s |
| Termux file create + verify (`device-e2e-file-create`) | PASS | `write_file` created `test_verify_e2e.txt`, `exec_shell cat` returned `HELLO_E2E` (2026-05-28) |
| Termux mini-project workflow (`device-e2e-project-workflow`) | PASS | create/edit `test_e2e_project/hello.txt`, verify contents, `pwd && ls` (2026-05-28) |
| MCP demo tool (`mcp__demo__echo`) | PASS | `device-e2e-mcp.ps1` → `MCP_E2E` via LAN HTTP JSON-RPC (2026-05-28) |
| Android project ZIP export+share (`device-e2e-zip-export`) | PASS | Headless probe: export zip + share callback `file_shared` (2026-05-28) |
| Android project ZIP import headless (`device-e2e-zip-import`) | PASS | `import_probe_marker.txt` extracted into workspace (2026-05-28) |
| Android project ZIP import (system picker UI) | Manual | `docs/ZIP_IMPORT_UI_TEST.md`; MIME `octet-stream` fix in picker |
| Termux `delete_file` (`device-e2e-delete-file`) | PASS | create → `delete_file` → `exec_shell` verify `/data/GONE` (2026-05-29) |
| Skills bundle (21 skills) | PASS | All `SKILL.md` on device; full-body injection enabled (2026-05-28) |
| PC mDNS discovery | Intermittent | `.pc_discovery_probe_running` can stick; requires `deepseek-pc-host` on `0.0.0.0:8787`, same Wi‑Fi |

Run: `. .\tools\android\env.ps1; .\scripts\device-full-verify.ps1 -Serial RFCNC0PWD4E`

## UI / logic fixes (this checkpoint)

| Issue | Fix |
|-------|-----|
| Chat opens at **first** message | Timeline uses `column-reverse` + scroll-to-bottom on load/thread switch (`chat_scroll.rs`) |
| Tools don't run in user chat | Termux file tools via RUN_COMMAND; inline JSON tool-call parsing; Termux workspace preferred on load |
| Termux lost after app restart | `TermuxResultReceiver` init data dir; pending Termux callback restore; re-activate workspace connection on load |

## UI / logic fixes (previous)

| Issue | Fix |
|-------|-----|
| **#8** Work log «выполняется» forever | `MobileTimelineState::seal_open_work_items()` on turn end, tool finish, restore; status rows move to **done**; badge «Ход работы … выполняется» clears when idle |
| **#6** `Failed to restore saved timeline: EOF` | Corrupt/empty runtime JSON skipped; benign EOF → fresh timeline, no error banner |
| «98 ходов» on «привет» | Reasoning stream merged into one **Reasoning** row |
| Duplicate Termux continuation events | Removed second replay of `result.events` in `lib.rs` |
| Stuck «Ожидание ответа Android» | JNI/UI bridge sync for Termux/PC/picker wait flags |
| Stuck «Работаю…» | `is_loading` cleared on turn complete, approval continuation, Termux continuation, API timeout (125 s) |
| Noisy work log | Skip internal statuses (`streaming opened/completed`, duplicate Started/Finished cards) |

## Verified on device (2026-05-28)

| Flow | Status |
|------|--------|
| Chat PONG (agent turn) | PASS via `agent_turn_probe` / manual chat |
| Termux pwd (quick template / probe) | PASS when Termux configured |
| Setup Termux buttons | Install/open F-Droid, RUN_COMMAND probe from setup screen |

**Manual testing:** do **not** run `device-*-verify.ps1` with force-stop while using the app interactively — probes are for scripted E2E only.

## Header chips (M·Auto/Flash/Pro, A·agent/plan/yolo, T·thinking)

| Chip | Works? | How |
|------|--------|-----|
| **M·model** | Yes | Saves `config.json` on tap; each chat turn calls `load_config_for_agent_turn()` |
| **A·execution** | Yes | Same save path; Plan disables tools in core |
| **T·thinking** | Yes | Saved to config; engine `ModelRouter` applies level for Pro/auto routes |

Engine selects API model per turn via `ModelRouter::route_prompt` (Flash/Pro/Auto), not only the static `config.model` string.

## Real-tool test (Termux pwd)

Script (does **not** force-stop by default):

```powershell
. .\tools\android\env.ps1
.\scripts\device-termux-pwd-probe.ps1 -Serial RFCNC0PWD4E
```

Uses isolated thread `__deepseek_adb_probe__`, YOLO auto-approve, built-in pwd prompt.  
Keep app in foreground ~2 min. Expect `.agent_turn_probe_result` line `PASS termux_tool …`.

Last automated run: **no result file** (app background or API hang) — re-run after install with app open.

## Manual verification still required

- [ ] OPEN_DOCUMENT picker → chat attachment (picker disabled in preview coordinator — use workspace path / adb)
- [ ] Import ZIP via system picker once (`docs/ZIP_IMPORT_UI_TEST.md`)
- [x] Export project ZIP (headless PASS)
- [x] Chat: «привет» → work log ~2 significant steps (Чат 4)
- [ ] Chat: «выполни pwd в termux» → approval → Termux → continuation (user + agent mode)
- [ ] PC Host pairing from phone (same LAN)
- [ ] All bottom-nav sections open without crash

## Known product gaps (not blocking chat)

- MCP external execution hardening
- Release APK signing
- LSP / symbol search (roadmap)
