use crate::locale::{pick, AppLanguage};
use crate::skills_state::SkillsUiState;
use crate::ui_layout::screen_layout;
use dioxus::prelude::*;

pub fn skills_panel(lang: AppLanguage, mut state: Signal<SkillsUiState>) -> Element {
    let layout = screen_layout();
    let mut loaded = use_signal(|| false);
    if !*loaded.peek() {
        state.write().refresh();
        loaded.set(true);
    }

    let skills = state.read().registry.skills.clone();
    let error = state.read().last_error.clone();
    let enabled_count = state.read().enabled_count();

    let skill_cards: Vec<Element> = skills.iter().map(|s| {
        let name = s.name.clone();
        let desc = s.description.clone();
        let enabled = s.enabled;
        let file_count = s.files.len();

        rsx! {
            div {
                key: "{name}",
                background_color: "#111827",
                border: "1px solid #1f2937",
                border_radius: "12px",
                padding: "10px",
                display: "flex",
                flex_direction: "column",
                gap: "4px",

                div {
                    display: "flex",
                    justify_content: "space_between",
                    align_items: "center",

                    div {
                        font_size: "13px",
                        font_weight: "bold",
                        color: "white",
                        "{name}"
                    }
                    button {
                        background_color: if enabled { "#16a34a" } else { "#374151" },
                        border: "none",
                        border_radius: "8px",
                        padding: "4px 12px",
                        color: "white",
                        font_size: "12px",
                        font_weight: "bold",
                        onclick: move |_| state.write().toggle_skill(&name, !enabled),
                        if enabled { "ON" } else { "OFF" }
                    }
                }

                div { color: "#9ca3af", font_size: "12px", "{desc}" }

                if file_count > 0 {
                    div { color: "#6b7280", font_size: "11px", "{file_count} companion file(s)" }
                }
            }
        }
    }).collect();

    rsx! {
        div {
            background_color: "#111827",
            color: "white",
            border: "1px solid #374151",
            border_radius: "16px",
            padding: "16px",
            display: "flex",
            flex_direction: "column",
            gap: "12px",

            div {
                display: "flex",
                justify_content: "space_between",
                align_items: "center",

                div {
                    font_size: "{layout.header_title_font}",
                    font_weight: "bold",
                    {pick(lang, "Навыки", "Skills")}
                    " ({skills.len()})"
                }
                div {
                    color: "#16a34a",
                    font_size: "{layout.subtitle_font}",
                    "{pick(lang, \"активно\", \"active\")}: {enabled_count}"
                }
            }

            if let Some(ref e) = error {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #991b1b",
                    border_radius: "8px",
                    padding: "8px",
                    color: "#fca5a5",
                    font_size: "12px",
                    "{e}"
                }
            }

            if skills.is_empty() {
                div {
                    style: "color:#9ca3af;font-size:{layout.body_font};line-height:1.5;padding:12px 0;display:flex;flex-direction:column;gap:8px;",
                    p {
                        style: "margin:0;",
                        {pick(
                            lang,
                            "Навыки — это папки с SKILL.md (инструкции для агента, как в Cursor).",
                            "Skills are folders with SKILL.md — agent instructions, similar to Cursor.",
                        )}
                    }
                    p {
                        style: "margin:0;color:#6b7280;font-size:{layout.subtitle_font};",
                        {pick(
                            lang,
                            "На телефоне: files/deepseek-mobile/skills/<имя>/SKILL.md",
                            "On phone: files/deepseek-mobile/skills/<name>/SKILL.md",
                        )}
                    }
                    p {
                        style: "margin:0;color:#6b7280;font-size:{layout.subtitle_font};",
                        {pick(
                            lang,
                            "Или скопируйте из ~/.cursor/skills на ПК через adb push.",
                            "Or adb push from ~/.cursor/skills on your PC.",
                        )}
                    }
                    div {
                        style: "background:#0b1220;border:1px solid #273244;border-radius:12px;padding:10px;color:#cbd5e1;font-size:{layout.subtitle_font};",
                        {pick(
                            lang,
                            "Быстрая команда: adb push <папка-навыка> /sdcard/Android/data/com.deepseek.mobile/files/deepseek-mobile/skills/",
                            "Quick command: adb push <skill-folder> /sdcard/Android/data/com.deepseek.mobile/files/deepseek-mobile/skills/",
                        )}
                    }
                }
            }

            div {
                display: "flex",
                flex_direction: "column",
                gap: "8px",
                {skill_cards.into_iter()}
            }
        }
    }
}
