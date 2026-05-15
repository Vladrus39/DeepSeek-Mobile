mod mobile_drawer;
mod pc_pairing_manager;
mod pc_pairing_panel;
mod pc_pairing_state;

use deepseek_mobile_core::{Config, DeepSeekCore, Message};
use dioxus::prelude::*;
use mobile_drawer::{mobile_drawer, CockpitSection};

fn main() {
    dioxus_mobile::launch(app);
}

fn app() -> Element {
    let mut messages = use_signal(Vec::<(String, String)>::new);
    let mut input = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut drawer_open = use_signal(|| false);
    let active_section = use_signal(|| CockpitSection::Chat);

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

            {mobile_drawer(drawer_open(), active_section())}

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

                if messages().is_empty() {
                    div {
                        color: "#9ca3af",
                        text_align: "center",
                        margin_top: "32px",
                        white_space: "pre-wrap",
                        "Ask DeepSeek to build, inspect, fix, test or deploy a project.\nOpen the drawer for PC Host, Files, Terminal, Git and Settings."
                    }
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
            }

            div {
                display: "flex",
                gap: "8px",
                margin_top: "12px",

                input {
                    flex: "1",
                    background_color: "#1f2937",
                    color: "white",
                    padding: "12px",
                    border: "1px solid #4b5563",
                    border_radius: "999px",
                    placeholder: "Ask anything...",
                    oninput: move |e| input.set(e.value()),
                }

                button {
                    background_color: "#3b82f6",
                    color: "white",
                    padding: "0 20px",
                    border_radius: "999px",
                    disabled: is_loading(),
                    onclick: move |_| {
                        let prompt = input();
                        if prompt.is_empty() { return; }

                        messages.push(("user".to_string(), prompt.clone()));
                        input.set(String::new());
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