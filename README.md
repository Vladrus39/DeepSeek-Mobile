# DeepSeek-Mobile

Mobile-first **DeepSeek Coding Agent** для Android с опциональным PC-host для тяжёлых задач через локальную сеть.

Проект уже содержит рабочее ядро агента, approval-flow, PC gateway, snapshots, Git/GitHub tools, diagnostics и мобильный cockpit UI. Это уже не макет, но ещё не финальная v1-сборка: часть экранов и интеграций существует, но не вся из них уже замкнута до production-ready end-to-end сценария.

## Что уже реально работает

- Чат с live streaming и reasoning delta
- Session/runtime persistence и continuation после approval
- Файловые инструменты, `apply_patch`, shell/git/web/GitHub tools
- Snapshots до destructive tools и после успешных turn’ов
- PC gateway: pairing ZIP, mDNS discovery, failover по endpoint’ам, health/logs, auth token
- Сохранение активного PC workspace после pairing и его использование в следующих turn’ах
- Diagnostics после edits: Rust, TypeScript и Python для local/Termux/PC путей
- Diagnostics metadata сохраняется в session и inject’ится в следующий model turn
- Native Android bridge contracts: document picker, PC discovery, share/terminal events, Termux RUN_COMMAND adapter and `exec_shell` native request queue
- Мобильные панели: chat, approvals, files, snapshots, diagnostics, PC host, terminal, git, settings
- Онбординг и сохранение настроек DeepSeek/GitHub

## Что ещё не доведено до конца

- Финальная Android host integration для native bridge
- Финальный Termux result lifecycle: Android host drain → Termux callback → final `exec_shell` output back into the agent
- Persistent terminal sessions между перезапусками
- Remote snapshot path для PC workspace
- Реальная wiring-логика Git panel и auto-commit/push в engine lifecycle
- Замена демонстрационного diff preview в file panel на реальные project diffs
- Durable background tasks, runtime API, MCP/plugins/skills

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
cargo check --workspace --all-targets
cargo test --workspace
```

## Текущий статус

- `cargo check --workspace --all-targets` — проходит
- `cargo test --workspace` — проходит
- Последний локальный прогон: 97 mobile tests, 117 core tests, 2 pc-host tests

Подробности:

- `docs/PROJECT_AUDIT.md`
- `docs/MASTER_PLAN.md`
- `docs/ROADMAP.md`
- `docs/android_host_integration.md`
- `PROJECT_STATUS.md`
