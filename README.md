# DeepSeek-Mobile

Mobile-first **DeepSeek Coding Agent** для Android с опциональным PC-host для тяжёлых задач через локальную сеть.

Проект уже содержит рабочее ядро агента, approval-flow, PC gateway, snapshots, Git/GitHub tools, diagnostics, durable task records/artifacts/logs, PC running-task sync, MCP/skills registry, Termux workspace activation и мобильный cockpit UI. Это уже не макет, но ещё не финальная v1-сборка: оставшиеся риски в основном вокруг финальной Android-host интеграции, Android packaging, runtime SSE/live sync и release packaging.

## Что уже реально работает

- Чат с live streaming и reasoning delta
- Session/runtime persistence и continuation после approval
- Файловые инструменты, `apply_patch` operation-batch + unified diff, shell/git/web/GitHub tools
- Snapshots до destructive tools и после успешных turn’ов, включая PC-gateway snapshot RPC path
- PC gateway: pairing ZIP, mDNS discovery, failover по endpoint’ам, health/logs, auth token
- Сохранение активного PC workspace после pairing и его использование в следующих turn’ах
- Diagnostics после edits: Rust, TypeScript и Python для local/Termux/PC путей
- Diagnostics metadata сохраняется в session и inject’ится в следующий model turn
- Termux `exec_shell` Rust/mobile lifecycle: native request queue, callback correlation, `continue_termux_result`
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
- Полноценный MCP stdio spawn и вызов MCP tools в tool loop
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
- Последний локальный полный прогон: 137 mobile / 170 core / 3 pc-host tests
- Android SDK для этого репо: `tools/android/` (изолированно от `D:\Project V`)

Подробности:

- `docs/PHONE_PC_OPERATING_MODEL.md`
- `docs/PROJECT_AUDIT.md`
- `docs/MASTER_PLAN.md`
- `docs/ROADMAP.md`
- `docs/android_host_integration.md`
- `docs/UI_STATUS_AND_VERIFICATION.md`
- `PROJECT_STATUS.md`
