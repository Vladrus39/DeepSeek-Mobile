//! Compact chat header: workspace + thread actions + agent mode chips in one strip.

use crate::agent_mode_bar::agent_mode_chips_row;
use crate::locale::{pick, AppLanguage};
use crate::settings_state::SettingsFormState;
use crate::ui_layout::ScreenLayout;
use deepseek_mobile_core::config::{ExecutionMode, ModelMode, ThinkingLevel};
use dioxus::prelude::*;

pub fn chat_toolbar(
    lang: AppLanguage,
    layout: &ScreenLayout,
    project_title: &str,
    chat_title: &str,
    history_open: bool,
    settings: &SettingsFormState,
    on_new_chat: EventHandler<()>,
    on_clear_screen: EventHandler<()>,
    on_history_toggle: EventHandler<()>,
    on_execution_mode: EventHandler<ExecutionMode>,
    on_model_mode: EventHandler<ModelMode>,
    on_thinking_level: EventHandler<ThinkingLevel>,
) -> Element {
    let new_label = pick(lang, "Новый", "New");
    let clear_label = pick(lang, "Очистить", "Clear");
    let history_label = if history_open {
        pick(lang, "Скрыть", "Hide")
    } else {
        pick(lang, "История", "Hist")
    };
    let cap = layout.caption_font;
    let sub = layout.subtitle_font;

    rsx! {
        div {
            style: "{crate::ui_layout::chat_chrome_card_style()}",
            div {
                style: "display:flex;align-items:center;gap:6px;min-width:0;",
                span {
                    style: "color:#475569;font-size:{cap};flex-shrink:0;",
                    "📁"
                }
                div {
                    style: "min-width:0;flex:1;line-height:1.25;",
                    div {
                        style: "color:#f1f5f9;font-size:{sub};font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                        "{project_title}"
                    }
                    div {
                        style: "color:#94a3b8;font-size:{cap};white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                        "💬 {chat_title}"
                    }
                }
                div {
                    style: "display:flex;gap:4px;flex-shrink:0;",
                    button {
                        style: "{crate::ui_layout::chrome_action_btn_style(true)}",
                        title: "{history_label}",
                        onclick: move |_| on_history_toggle.call(()),
                        "{history_label}"
                    }
                    button {
                        style: "{crate::ui_layout::chrome_action_btn_style(false)}",
                        title: "{new_label}",
                        onclick: move |_| on_new_chat.call(()),
                        "+ {new_label}"
                    }
                    button {
                        style: "{crate::ui_layout::chrome_action_btn_style(false)}",
                        title: "{clear_label}",
                        onclick: move |_| on_clear_screen.call(()),
                        "⌫"
                    }
                }
            }
            {agent_mode_chips_row(
                lang,
                settings,
                on_execution_mode,
                on_model_mode,
                on_thinking_level,
            )}
        }
    }
}
