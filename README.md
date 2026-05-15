# DeepSeek-Mobile

Полноценный **DeepSeek Coding Agent** для Android.

Работает полностью локально на телефоне + DeepSeek API (V4-pro / Flash, 1M контекст).

## Основные возможности
- Полноценный чат с streaming reasoning
- Workspace Explorer
- Все инструменты оригинального TUI
- Режимы Plan / Agent / YOLO
- Auto mode (v4-pro / flash + thinking levels)

## Стек
- **Core**: Rust (transfer from DeepSeek-TUI)
- **UI**: Dioxus 0.7 (native Android)

## Быстрый старт
```bash
git clone https://github.com/Vladrus39/DeepSeek-Mobile.git
cd DeepSeek-Mobile
# Для Android:
dx serve --platform android
```

## Статус
Я (Grok) полностью контролирую проект и переношу core из оригинального DeepSeek-TUI.