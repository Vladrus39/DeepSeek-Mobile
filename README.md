# DeepSeek-Mobile

Mobile-first **DeepSeek Coding Agent** для Android с опциональным PC-host для тяжёлых задач через локальную сеть.

Проект уже содержит рабочее ядро агента, approval-flow, PC gateway, snapshots, Git/GitHub tools, diagnostics, durable task records, MCP/skills registry и мобильный cockpit UI. Это уже не макет, но ещё не финальная v1-сборка: оставшиеся риски в основном вокруг финальной Android-host интеграции, packaging и runtime API.

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
- Durable task records + queue lifecycle + mobile task manager UI
- Онбординг и сохранение настроек DeepSeek/GitHub

## Что ещё не доведено до конца

- Финальный Dioxus Android host adapter и ручная device/emulator verification bridge-контрактов
- Termux workspace selector и Android import/export completion
- Runtime HTTP/SSE API поверх durable task/runtime модели
- Artifacts/logs per durable task и более тесная связка task UI с PC-host running tasks
- Dev-server lifecycle, PC-host autostart/service installer
- Android/PC-host release notes, troubleshooting docs и packaging

## Стек

- **Core**: Rust
- **UI**: Dioxus 0.7
- **PC-host**: HTTP + SSE сервер на Rust

## Быстрый старт

```bash
git clone https://github.com/Vladrus39/DeepSeek-Mobile.git
cd DeepSeek-Mobile

# Android dev flow
dx serve --platform android

# Проверка на Windows/MSVC
cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
cargo +stable-x86_64-pc-windows-msvc test --workspace
```

## Текущий статус

- `cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets` — проходит
- `cargo +stable-x86_64-pc-windows-msvc test --workspace` — проходит
- Последний локальный полный прогон: 108 mobile tests, 152 core tests, 2 pc-host tests

Подробности:

- `docs/PROJECT_AUDIT.md`
- `docs/MASTER_PLAN.md`
- `docs/ROADMAP.md`
- `docs/android_host_integration.md`
- `PROJECT_STATUS.md`
