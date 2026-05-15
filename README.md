# DeepSeek-Mobile

Полноценный DeepSeek Coding Agent для Android на Dioxus + Rust Core.

## Цель
Перенос всего функционала оригинального DeepSeek-TUI (Auto mode, 1M контекст, v4-pro/flash, все инструменты, streaming reasoning, session rollback и т.д.) в мобильное приложение.

## Стек
- Core: Rust (shared crate)
- UI: Dioxus 0.7 Mobile
- Android: Scoped Storage + Shell execution

## Статус
В процессе полного переноса core.