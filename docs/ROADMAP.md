# DeepSeek-Mobile — Roadmap

## Этап 0: Foundation (1-2 дня)
- [x] Создать workspace структуру
- [x] Cargo.toml + dioxus.toml
- [ ] crates/core (выделение из оригинального DeepSeek-TUI)
- [ ] Базовый Dioxus Android проект с чатом

## Этап 1: Core Integration (3-5 дней)
- Перенос engine, agent loop, tool registry
- Tool calling (file read/write, shell exec на Android)
- Streaming + reasoning blocks

## Этап 2: Android Integration (4-7 дней)
- Scoped Storage + Storage Access Framework
- Shell execution через Android
- Permissions и background tasks

## Этап 3: Полноценный UI
- Workspace Explorer + Tree view
- Inline editing + Diff viewer
- Tool approval панель
- Sidebar (сессии, настройки)

## Этап 4+: Продвинутые фичи
- LSP diagnostics
- Sub-agents + MCP
- Voice input
- Полная миграция всех инструментов
