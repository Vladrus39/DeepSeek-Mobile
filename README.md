# DeepSeek-Mobile

Полноценный **DeepSeek Coding Agent** для Android.

Работает полностью локально на телефоне + DeepSeek API (V4-pro / Flash, 1M контекст).

## Основные возможности
- Полноценный чат с streaming reasoning
- Workspace Explorer (работа с реальными проектами на телефоне)
- Все инструменты оригинального TUI (file edit, shell, git, LSP diagnostics, rollback и т.д.)
- Режимы: Plan, Agent, YOLO, Auto
- Sub-agents + MCP
- Сессии и история

## Стек
- **Core**: Rust (переиспользуем из DeepSeek-TUI)
- **UI**: Dioxus 0.7 (native Android)
- **Backend**: Shared library

## Быстрый старт

```bash
# Клонировать
git clone https://github.com/Vladrus39/DeepSeek-Mobile.git
cd DeepSeek-Mobile

# Установка зависимостей (позже)
```
