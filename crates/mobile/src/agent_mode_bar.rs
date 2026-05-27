use crate::locale::{pick, AppLanguage};
use crate::settings_state::SettingsFormState;
use deepseek_mobile_core::config::{ExecutionMode, ModelMode, ThinkingLevel};
use dioxus::prelude::*;

/// One-line agent / model / thinking toggles (no section title).
pub fn agent_mode_chips_row(
    lang: AppLanguage,
    settings: &SettingsFormState,
    on_execution_mode: EventHandler<ExecutionMode>,
    on_model_mode: EventHandler<ModelMode>,
    on_thinking_level: EventHandler<ThinkingLevel>,
) -> Element {
    let tools_label = match settings.execution_mode {
        ExecutionMode::Plan => pick(lang, "план", "plan"),
        ExecutionMode::Agent => pick(lang, "агент", "agent"),
        ExecutionMode::Yolo => pick(lang, "yolo", "yolo"),
    };
    let tools_hint = match settings.execution_mode {
        ExecutionMode::Plan => pick(lang, "без инструментов", "no tools"),
        ExecutionMode::Agent => pick(lang, "инструменты+одобрение", "tools+approve"),
        ExecutionMode::Yolo => pick(lang, "инструменты авто", "tools auto"),
    };
    let mode_caption = pick(lang, "Режим", "Mode");
    let model_label = match settings.model_mode {
        ModelMode::Auto => "Auto",
        ModelMode::Flash => "Flash",
        ModelMode::Pro => "Pro",
    };
    let thinking_label = match settings.thinking_level {
        ThinkingLevel::Off => pick(lang, "выкл", "off"),
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "med",
        ThinkingLevel::High => "high",
        ThinkingLevel::Max => "max",
    };
    let next_mode = next_execution_mode(&settings.execution_mode);
    let next_model = next_model_mode(&settings.model_mode);
    let next_thinking = next_thinking_level(&settings.thinking_level);
    let chip_agent = crate::ui_layout::mode_chip_style("#1d4ed8", "#bfdbfe");
    let chip_model = crate::ui_layout::mode_chip_style("#5b21b6", "#c4b5fd");
    let chip_think = crate::ui_layout::mode_chip_style("#a16207", "#fde68a");

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:4px;margin-top:6px;min-width:0;",
            div {
                style: "color:#64748b;font-size:10px;line-height:1.2;",
                "{mode_caption}: {tools_hint}"
            }
            div {
                style: "display:flex;gap:4px;min-width:0;",
            button {
                style: "{chip_agent}",
                onclick: move |_| on_execution_mode.call(next_mode.clone()),
                title: "{tools_hint}",
                "A·{tools_label}"
            }
            button {
                style: "{chip_model}",
                onclick: move |_| on_model_mode.call(next_model.clone()),
                "M·{model_label}"
            }
            button {
                style: "{chip_think}",
                onclick: move |_| on_thinking_level.call(next_thinking.clone()),
                "T·{thinking_label}"
            }
            }
        }
    }
}

/// Full settings panel block with title (non-chat sections if needed later).
pub fn agent_mode_bar(
    lang: AppLanguage,
    settings: &SettingsFormState,
    on_execution_mode: EventHandler<ExecutionMode>,
    on_model_mode: EventHandler<ModelMode>,
    on_thinking_level: EventHandler<ThinkingLevel>,
) -> Element {
    let title = pick(lang, "Режим агента", "Agent mode");
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:4px;",
            div {
                style: "color:#64748b;font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:0.05em;",
                "{title}"
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

fn next_execution_mode(mode: &ExecutionMode) -> ExecutionMode {
    match mode {
        ExecutionMode::Plan => ExecutionMode::Agent,
        ExecutionMode::Agent => ExecutionMode::Yolo,
        ExecutionMode::Yolo => ExecutionMode::Plan,
    }
}

fn next_model_mode(mode: &ModelMode) -> ModelMode {
    match mode {
        ModelMode::Auto => ModelMode::Flash,
        ModelMode::Flash => ModelMode::Pro,
        ModelMode::Pro => ModelMode::Auto,
    }
}

fn next_thinking_level(level: &ThinkingLevel) -> ThinkingLevel {
    match level {
        ThinkingLevel::Off => ThinkingLevel::Low,
        ThinkingLevel::Low => ThinkingLevel::Medium,
        ThinkingLevel::Medium => ThinkingLevel::High,
        ThinkingLevel::High => ThinkingLevel::Max,
        ThinkingLevel::Max => ThinkingLevel::Off,
    }
}

#[cfg(test)]
mod tests {
    use super::{next_execution_mode, next_model_mode, next_thinking_level};
    use deepseek_mobile_core::config::{ExecutionMode, ModelMode, ThinkingLevel};

    #[test]
    fn mode_chips_cycle() {
        assert_eq!(
            next_execution_mode(&ExecutionMode::Plan),
            ExecutionMode::Agent
        );
        assert_eq!(next_model_mode(&ModelMode::Pro), ModelMode::Auto);
        assert_eq!(next_thinking_level(&ThinkingLevel::Max), ThinkingLevel::Off);
    }
}
