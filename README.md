# DeepSeek-Mobile

Mobile-first **DeepSeek Coding Agent** для Android — **полноценный агент на телефоне** в TUI-подобной модели работы, с **Termux** как основным executor для реальных проектов. **PC Host опционален** — для очень больших репозиториев, когда удобнее доводить работу на рабочей станции.

Каноническая формулировка продукта: [`docs/PRODUCT_POSITIONING.md`](docs/PRODUCT_POSITIONING.md).

Проект уже содержит рабочее ядро агента, approval-flow, Termux workspace, snapshots, Git/GitHub tools, diagnostics, MCP/skills, мобильный cockpit UI и **опциональный** PC gateway (pairing, tasks, terminal). Не финальная v1: Android device verification, MCP stdio на устройстве, release APK.

## Как устроен «полный агент» на телефоне

| Режим | Для чего |
|-------|----------|
| **Termux workspace** (главный) | shell, git, build, test — как на ПК в TUI |
| **Sandbox приложения** | чат, мелкие правки, ZIP import/export |
| **PC Host** (опция) | огромные проекты / desktop toolchains без установки в Termux |

## Что уже реально работает

- Чат с live streaming и reasoning delta
- Session/runtime persistence и continuation после approval
- Файловые инструменты, `apply_patch` operation-batch + unified diff, shell/git/web/GitHub tools
- Snapshots до destructive tools и после успешных turn’ов, включая PC-gateway snapshot RPC path
- Termux: Settings + онбординг, `exec_shell` через native bridge, continuation turn’а
- **Опционально** PC gateway: pairing ZIP, discovery, tasks/terminal/SSE (панель PC Host)
- Diagnostics после edits: Rust, TypeScript и Python для local/Termux/PC путей
- Diagnostics metadata сохраняется в session и inject’ится в следующий model turn
- Native Android bridge contracts: document picker, PC discovery, share/terminal events, Termux `RUN_COMMAND` adapter
- Мобильные панели: chat, approvals, files с real pending diffs и PC-aware browsing, snapshots, diagnostics, PC host, terminal, Git actions, tasks, MCP, skills, settings
- UI chrome: live API/PC chips, active workspace header, dynamic badges for approvals/diagnostics/Git/tasks/native waits, drawer + scrollable bottom navigation
- Durable task records + queue lifecycle + artifacts/log capture + mobile task manager UI
- Runtime HTTP task API on PC-host: task listing and per-task log retrieval
- Tasks panel syncs active PC-host running tasks via SSE (`stream_task_events`) and can stop them through PC Host
- Encrypted secrets storage (`secrets.enc` + `device.key`) for API/GitHub tokens
- Skills context injection into engine turns; MCP HTTP connect + declared tools fallback
- Android host coordinator (`DeepSeekMobileHostCoordinator`) + Rust `android_host` drain loop
- Termux workspace selector in Settings: validates absolute Termux paths, persists config and activates the Termux runtime workspace
- Core ZIP workspace import/export helpers with path-traversal protection and `.deepseek-mobile` metadata exclusion
- Files panel project import/export UI: Android archive picker import into phone workspace, ZIP export and native share queue
- Онбординг и сохранение настроек DeepSeek/GitHub

## Что ещё не доведено до конца

- JNI-связка Dioxus Android activity с `NativeBridgeBindings` и device/emulator verification
- Финальная визуальная проверка на Android через Dioxus CLI/emulator/device
- MCP stdio session reuse и on-device verification MCP tools
- Dev-server lifecycle, PC-host autostart/service installer
- Android/PC-host release packaging (signed APK/installer)

## Стек

- **Core**: Rust
- **UI**: Dioxus 0.7
- **PC-host**: HTTP + SSE сервер на Rust

## Быстрый старт

```bash
git clone https://github.com/Vladrus39/DeepSeek-Mobile.git
cd DeepSeek-Mobile

# Android toolchain (изолированно в tools/android/, не D:\Project V)
. .\tools\android\env.ps1   # PowerShell

# Android dev flow (когда установлены dx + NDK — см. tools/android/DOWNLOAD_BUDGET.md)
dx serve --platform android

# Проверка на Windows/MSVC
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace
```

## Текущий статус

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — проходит
- `cargo +stable-x86_64-pc-windows-msvc test --workspace` — проходит
- Последний локальный полный прогон: 140 mobile / 178 core / 3 pc-host tests
- Android SDK для этого репо: `tools/android/` (изолированно от `D:\Project V`)

Подробности:

- `docs/PRODUCT_POSITIONING.md` — phone-first, PC optional
- `docs/PHONE_PC_OPERATING_MODEL.md`
- `docs/CAPABILITY_MATRIX.md` — honest limits vs Cursor / full-phone control
- `docs/PROJECT_AUDIT.md`
- `docs/MASTER_PLAN.md`
- `docs/ROADMAP.md`
- `docs/android_host_integration.md`
- `docs/UI_STATUS_AND_VERIFICATION.md`
- `PROJECT_STATUS.md`
