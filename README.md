# DeepSeek-Mobile

Полноценный DeepSeek Coding Agent для Android на Dioxus + Rust.

Работает локально на телефоне, использует DeepSeek API (v4-pro / v4-flash, 1M контекст).

## Стек
- Core: Rust (перенесён из Hmbown/DeepSeek-TUI)
- UI: Dioxus 0.7 (native Android)
- Agent: Полный tool calling, auto-mode, streaming

## Цель
Сохранить 95%+ функционала оригинального TUI в удобном touch-интерфейсе.