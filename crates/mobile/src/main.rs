mod chat_attachment;
mod cockpit_section_panel;
mod mobile_drawer;
mod pc_pairing_manager;
mod pc_pairing_panel;
mod pc_pairing_state;

use chat_attachment::{ChatAttachmentDraft, ChatComposerState};
use cockpit_section_panel::cockpit_section_panel;
use deepseek_mobile_core::{Config, DeepSeekCore, Message};
use dioxus::prelude::*;
use mobile_drawer::{mobile_drawer, CockpitSection};
use pc_pairing_state::PcPairingUiState;

fn main() {
    dioxus_mobile::launch(app);
}

fn app() -> Element {
    let mut messages = use_signal(Vec::<(String, String)>::new);
    let mut input = use_signal(String::new);
    let mut composer = use_signal(ChatComposerState::default);
    let mut is_loading = use_signal(|| false);
    let mut drawer_open = use_signal(|| false);
    let mut active_section = use_signal(|| CockpitSection::Chat);
    let pc_pairing_state = use_signal(PcPairingUiState::default);

    rsx! {
        div {
            background_color: "#0f0f0f",
            color: "white",
            height: "100vh",
            padding: "16px",
            display: "flex",
            flex_direction: "column",
            position: "relative",
            overflow: "hidden",

            {mobile_drawer(
                drawer_open(),
                active_section(),
                move |section| {
                    active_section.set(section);
                    drawer_open.set(false);
                }
            )}

            div {
                display: "flex",
                align_items: "center",
                justify_content: "space-between",
                margin_bottom: "12px",

                button {
                    background_color: "#1f2937",
                    color: "white",
                    width: "44px",
                    height: "44px",
                    border_radius: "999px",
                    border: "1px solid #374151",
                    onclick: move |_| drawer_open.set(!drawer_open()),
                    "☰"
                }

                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    div {
                        font_size: "18px",
                        font_weight: "bold",
                        "DeepSeek Mobile"
                    }
                    div {
                        color: "#9ca3af",
                        font_size: "12px",
                        "{active_section().subtitle()}"
                    }
                }

                div {
                    background_color: "#111827",
                    color: "#d1d5db",
                    border: "1px solid #374151",
                    border_radius: "999px",
                    padding: "8px 10px",
                    font_size: "12px",
                    "API"
                }
            }

            div {
                flex: "1",
                background_color: "#111827",
                padding: "12px",
                border_radius: "18px",
                overflow_y: "auto",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                if active_section() == CockpitSection::Chat {
                    if messages().is_empty() {
                        {cockpit_section_panel(CockpitSection::Chat, &pc_pairing_state())}
                    }

                    for (role, content) in messages() {
                        div {
                            background_color: if role == "user" { "#2563eb" } else { "#1f2937" },
                            padding: "10px 14px",
                            border_radius: "14px",
                            max_width: "85%",
                            align_self: if role == "user" { "flex-end" } else { "flex-start" },
                            white_space: "pre-wrap",
                            "{content}"
                        }
                    }

                    if is_loading() {
                        div {
                            color: "#9ca3af",
                            "Thinking with DeepSeek..."
                        }
                    }
                } else {
                    {cockpit_section_panel(active_section(), &pc_pairing_state())}
                }
            }

            if !composer().attachments.is_empty() {
                div {
                    margin_top: "8px",
                    background_color: "#111827",
                    border: "1px solid #374151",
                    border_radius: "14px",
                    padding: "8px 10px",
                    color: "#d1d5db",
                    font_size: "12px",
                    "{composer().attachment_summary()}"
                }
            }

            div {
                display: "flex",
                gap: "8px",
                margin_top: "12px",
                align_items: "center",

                button {
                    background_color: "#1f2937",
                    color: "white",
                    width: "44px",
                    height: "44px",
                    border_radius: "999px",
                    border: "1px solid #4b5563",
                    onclick: move |_| {
                        let mut next = composer();
                        let index = next.attachments.len() + 1;
                        next.add_attachment(ChatAttachmentDraft::new_document(
                            format!("draft-doc-{}", index),
                            format!("document-{}.pdf", index),
                        ));
                        composer.set(next);
                    },
                    "+"
                }

                input {
                    flex: "1",
                    background_color: "#1f2937",
                    color: "white",
                    padding: "12px",
                    border: "1px solid #4b5563",
                    border_radius: "999px",
                    placeholder: "Message DeepSeek...",
                    value: "{input}",
                    oninput: move |e| {
                        let value = e.value();
                        input.set(value.clone());
                        let mut next = composer();
                        next.draft_text = value;
                        composer.set(next);
                    },
                }

                button {
                    background_color: "#3b82f6",
                    color: "white",
                    padding: "0 20px",
                    height: "44px",
                    border_radius: "999px",
                    disabled: is_loading() || !composer().has_content(),
                    onclick: move |_| {
                        let draft = composer();
                        if !draft.has_content() { return; }

                        let mut prompt = draft.draft_text.clone();
                        if !draft.attachments.is_empty() {
                            if !prompt.is_empty() {
                                prompt.push_str("\n\n");
                            }
                            prompt.push_str("Attachments:\n");
                            for attachment in &draft.attachments {
                                prompt.push_str("- ");
                                prompt.push_str(&attachment.display_name);
                                prompt.push('\n');
                            }
                        }

                        messages.push(("user".to_string(), prompt.clone()));
                        input.set(String::new());
                        composer.set(ChatComposerState::default());
                        is_loading.set(true);

                        spawn(async move {
                            let config = Config::default();
                            let core = DeepSeekCore::new(config);
                            
                            let chat_messages: Vec<Message> = messages()
                                .into_iter()
                                .filter(|(role, _)| role != "system")
                                .map(|(role, content)| Message { role, content })
                                .collect();
                            
                            match core.process_with_messages(chat_messages).await {
                                Ok(response) => messages.push(("assistant".to_string(), response)),
                                Err(e) => messages.push(("assistant".to_string(), format!("Error: {}", e))),
                            }
                            is_loading.set(false);
                        });
                    },
                    "Send"
                }
            }
        }
    }
}