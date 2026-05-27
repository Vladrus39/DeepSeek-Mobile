//! One-tap prompts for common coding-agent workflows.

use crate::locale::{pick, AppLanguage};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickAction {
    pub label_ru: &'static str,
    pub label_en: &'static str,
    pub prompt: &'static str,
}

pub const QUICK_ACTIONS: &[QuickAction] = &[
    QuickAction {
        label_ru: "План",
        label_en: "Plan",
        prompt: "Режим плана: проанализируй активную рабочую область и предложи следующие шаги. Не выдумывай вывод команд — только контекст.",
    },
    QuickAction {
        label_ru: "Termux pwd",
        label_en: "Termux pwd",
        prompt: "Выполни pwd и ls -la в активной Termux-области через exec_shell и кратко опиши окружение.",
    },
    QuickAction {
        label_ru: "Git status",
        label_en: "Git status",
        prompt: "Выполни git status в активной области и опиши ветку, staged-изменения и риски.",
    },
    QuickAction {
        label_ru: "Тесты",
        label_en: "Run tests",
        prompt: "Запусти тесты проекта (для Rust предпочитай cargo test --workspace). Кратко перечисли падения с путями.",
    },
    QuickAction {
        label_ru: "Структура",
        label_en: "Structure",
        prompt: "Покажи структуру проекта верхнего уровня: точки входа, сборка, конфиги.",
    },
    QuickAction {
        label_ru: "Диагностика",
        label_en: "Diagnostics",
        prompt: "По последней диагностике предложи минимальные безопасные исправления.",
    },
    QuickAction {
        label_ru: "Открыть на PC",
        label_en: "Open on PC",
        prompt: "Если активен PC Host — open_path на корень проекта. Иначе скажи, что активен Termux, и list_dir.",
    },
];

pub fn chat_quick_actions_bar(
    lang: AppLanguage,
    on_select: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    let title = pick(lang, "Быстрые шаблоны", "Quick templates");
    let hint = pick(
        lang,
        "Подставляют текст в поле. Отправку подтверждаете отдельно.",
        "They fill the composer. You still confirm Send.",
    );
    let close = pick(lang, "Скрыть", "Hide");
    rsx! {
        div {
            background_color: "#0b1220",
            border: "1px solid #1f2937",
            border_radius: "18px",
            padding: "10px",
            display: "flex",
            flex_direction: "column",
            gap: "8px",
            margin_top: "8px",

            div {
                display: "flex",
                justify_content: "space-between",
                align_items: "center",
                gap: "8px",

                div {
                    div {
                        color: "#e5e7eb",
                        font_size: "13px",
                        font_weight: "800",
                        "{title}"
                    }
                    div {
                        color: "#6b7280",
                        font_size: "11px",
                        "{hint}"
                    }
                }
                button {
                    background_color: "transparent",
                    color: "#93c5fd",
                    border: "1px solid #1d4ed8",
                    border_radius: "999px",
                    padding: "5px 10px",
                    font_size: "12px",
                    onclick: move |_| on_close.call(()),
                    "{close}"
                }
            }

            div {
                display: "flex",
                gap: "6px",
                overflow_x: "auto",
                padding_bottom: "4px",
                style: "-webkit-overflow-scrolling: touch;",

                for action in QUICK_ACTIONS {
                    button {
                        flex_shrink: "0",
                        background_color: "#1f2937",
                        color: "#e5e7eb",
                        border: "1px solid #4b5563",
                        border_radius: "999px",
                        padding: "8px 12px",
                        font_size: "clamp(0.75rem, 2.5vw, 0.85rem)",
                        white_space: "nowrap",
                        min_height: "40px",
                        onclick: {
                            let prompt = action.prompt.to_string();
                            move |_| on_select.call(prompt.clone())
                        },
                        {pick(lang, action.label_ru, action.label_en)}
                    }
                }
            }
        }
    }
}
