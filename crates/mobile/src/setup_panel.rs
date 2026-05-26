//! First login: checklist + fields + one Continue button (no separate Termux install button).

use crate::locale::{tr, AppLanguage, Tr};
use crate::setup_status::SetupSnapshot;
use crate::ui_layout::{centered_card_style, screen_layout};
use dioxus::prelude::*;

pub fn setup_panel(
    mut lang: Signal<AppLanguage>,
    snapshot: SetupSnapshot,
    mut api_key_draft: Signal<String>,
    mut termux_path_draft: Signal<String>,
    validation_error: Signal<Option<String>>,
    on_continue: EventHandler<()>,
    on_sandbox_only: EventHandler<()>,
) -> Element {
    let layout = screen_layout();
    let card_style = centered_card_style(&layout);

    rsx! {
        div {
            style: "display:flex;flex-direction:column;justify-content:center;align-items:center;min-height:100vh;width:100%;max-width:100vw;box-sizing:border-box;background:#0f0f0f;color:white;padding:{layout.viewport_padding};gap:clamp(12px,3vw,20px);overflow-y:auto;",

            div {
                style: "{card_style} display:flex;flex-direction:column;gap:clamp(10px,2.5vw,16px);",

                div {
                    style: "display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap;",
                    div {
                        style: "font-size:{layout.title_font};font-weight:bold;",
                        "{tr(lang(), Tr::SetupTitle)}"
                    }
                    button {
                        style: "background:#1f2937;color:#e5e7eb;border:1px solid #4b5563;border-radius:999px;padding:6px 14px;font-size:{layout.body_font};min-height:44px;min-width:44px;",
                        onclick: move |_| {
                            let next = lang().toggle();
                            lang.set(next);
                            let _ = crate::locale::save_ui_language(next);
                        },
                        "{lang().label()}"
                    }
                }

                p {
                    style: "color:#9ca3af;font-size:{layout.subtitle_font};line-height:1.45;margin:0;",
                    "{tr(lang(), Tr::SetupSubtitle)}"
                }

                div {
                    style: "background:#111827;border:1px solid #374151;border-radius:16px;padding:clamp(12px,3vw,16px);display:flex;flex-direction:column;gap:8px;",
                    {check_row(lang(), Tr::CheckApi, snapshot.api_ok)}
                    {check_row(lang(), Tr::CheckAgent, snapshot.agent_mode_ok)}
                    {check_row(lang(), Tr::CheckTermux, snapshot.termux_ok)}
                }

                label {
                    style: "font-size:{layout.body_font};font-weight:bold;",
                    "{tr(lang(), Tr::SetupApiLabel)}"
                }
                input {
                    style: "width:100%;box-sizing:border-box;background:#1f2937;color:white;padding:{layout.button_padding};border:1px solid #4b5563;border-radius:14px;font-size:{layout.body_font};min-height:48px;",
                    placeholder: "{tr(lang(), Tr::SetupApiPlaceholder)}",
                    value: "{api_key_draft}",
                    oninput: move |e| api_key_draft.set(e.value()),
                }

                label {
                    style: "font-size:{layout.body_font};font-weight:bold;margin-top:4px;",
                    "{tr(lang(), Tr::SetupTermuxLabel)}"
                }
                p {
                    style: "color:#6b7280;font-size:{layout.subtitle_font};margin:0;line-height:1.4;",
                    "{tr(lang(), Tr::SetupTermuxHint)}"
                }
                input {
                    style: "width:100%;box-sizing:border-box;background:#1f2937;color:white;padding:{layout.button_padding};border:1px solid #4b5563;border-radius:14px;font-size:{layout.body_font};min-height:48px;",
                    value: "{termux_path_draft}",
                    oninput: move |e| termux_path_draft.set(e.value()),
                }

                if let Some(err) = validation_error() {
                    div { style: "color:#fca5a5;font-size:{layout.subtitle_font};", "{err}" }
                }

                button {
                    style: "width:100%;box-sizing:border-box;background:#3b82f6;color:white;padding:{layout.button_padding};border-radius:14px;border:none;font-weight:bold;font-size:{layout.body_font};min-height:48px;margin-top:8px;",
                    onclick: move |_| on_continue.call(()),
                    "{tr(lang(), Tr::SetupContinue)}"
                }

                button {
                    style: "width:100%;box-sizing:border-box;background:transparent;color:#9ca3af;padding:12px;border-radius:12px;border:1px solid #4b5563;font-size:{layout.subtitle_font};min-height:44px;",
                    onclick: move |_| on_sandbox_only.call(()),
                    "{tr(lang(), Tr::SetupSandboxOnly)}"
                }
            }
        }
    }
}

fn check_row(lang: AppLanguage, key: Tr, ok: bool) -> Element {
    let (icon, color) = if ok {
        ("✓", "#10b981")
    } else {
        ("✗", "#f87171")
    };
    let layout = screen_layout();
    rsx! {
        div {
            style: "display:flex;justify-content:space-between;align-items:center;gap:8px;font-size:{layout.body_font};",
            span { "{tr(lang, key)}" }
            span { style: "color:{color};font-weight:bold;", "{icon}" }
        }
    }
}
