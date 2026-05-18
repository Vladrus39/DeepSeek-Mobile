# DeepSeek-Mobile

Полноценный **DeepSeek Coding Agent** для Android.

Работает локально на телефоне + DeepSeek API (V4-pro / Flash, 1M контекст).
Опциональный PC-host для тяжёлых задач через локальную сеть.

## Возможности

- Полноценный чат со streaming reasoning (текст + reasoning блоки)
- Workspace Explorer с деревом файлов, навигацией по папкам, предпросмотром
- Все инструменты оригинального TUI: файловые операции, git, shell, patch, snapshots
- Режимы Plan / Agent / YOLO с настраиваемым approval
- PC Gateway — управление компьютером с телефона (LAN, mDNS, pairing ZIP)
- GitHub интеграция: токен, репозиторий, PR, issues, авто-коммиты
- Терминальные сессии на Android / PC-host
- Web-инструменты: fetch URL, поиск через DuckDuckGo (с approval)
- Снэпшоты workspace с авто-восстановлением и rollback
- Git UI: статус, diff, commit, push, pull, branch
- Онбординг с настройкой API-ключа при первом запуске
- Нижняя навигация: Chat / Files / Terminal / Git / Settings

## Стек

- **Core**: Rust (перенесён из DeepSeek-TUI)
- **UI**: Dioxus 0.7 (native Android)
- **PC-host**: HTTP + SSE сервер на Rust

## Быстрый старт

```bash
git clone https://github.com/Vladrus39/DeepSeek-Mobile.git
cd DeepSeek-Mobile
# Для Android:
dx serve --platform android
# Проверка сборки (Windows MSVC):
cargo +stable-x86_64-pc-windows-msvc check --workspace
```

## Статус проекта

Проект в активной разработке. Основной функционал перенесён из DeepSeek-TUI.
Сборка: `cargo check --workspace` — 0 errors. 87 mobile тестов проходят.

Подробнее: `docs/MASTER_PLAN.md`, `docs/ROADMAP.md`, `PROJECT_STATUS.md`.
