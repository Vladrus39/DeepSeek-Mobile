use crate::settings_state::save_config;
use deepseek_mobile_core::config::Config;
use dioxus::prelude::*;

/// Full-screen onboarding panel shown on first launch when no API key is configured.
pub fn onboarding_panel(on_complete: EventHandler<String>) -> Element {
    let mut api_key_input = use_signal(String::new);
    let mut validation_error = use_signal(|| None::<String>);

    rsx! {
        div {
            background_color: "#0f0f0f",
            color: "white",
            height: "100vh",
            width: "100%",
            display: "flex",
            flex_direction: "column",
            justify_content: "center",
            align_items: "center",
            padding: "24px",
            gap: "20px",

            div {
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                gap: "8px",

                div {
                    font_size: "48px",
                    "\u{1f9e0}"
                }
                div {
                    font_size: "28px",
                    font_weight: "bold",
                    "DeepSeek Mobile"
                }
                div {
                    color: "#9ca3af",
                    font_size: "14px",
                    text_align: "center",
                    max_width: "320px",
                    "AI coding agent for Android. Full tool stack, PC gateway, 1M context."
                }
            }

            div {
                background_color: "#111827",
                border: "1px solid #374151",
                border_radius: "18px",
                padding: "20px",
                width: "100%",
                max_width: "380px",
                display: "flex",
                flex_direction: "column",
                gap: "14px",

                div {
                    font_size: "16px",
                    font_weight: "bold",
                    "Connect to DeepSeek API"
                }
                div {
                    color: "#9ca3af",
                    font_size: "12px",
                    "Enter your API key to get started. Your key is stored locally and never shared."
                }

                input {
                    background_color: "#1f2937",
                    color: "white",
                    padding: "14px",
                    border: if validation_error().is_some() { "1px solid #ef4444" } else { "1px solid #4b5563" },
                    border_radius: "14px",
                    font_size: "14px",
                    placeholder: "sk-...",
                    value: "{api_key_input}",
                    oninput: move |e| {
                        api_key_input.set(e.value());
                        validation_error.set(None);
                    },
                }

                if let Some(error) = validation_error() {
                    div {
                        color: "#fca5a5",
                        font_size: "12px",
                        "{error}"
                    }
                }

                button {
                    background_color: if api_key_input().trim().len() >= 8 { "#3b82f6" } else { "#374151" },
                    color: "white",
                    padding: "14px",
                    border_radius: "14px",
                    border: "none",
                    font_size: "16px",
                    font_weight: "bold",
                    disabled: api_key_input().trim().len() < 8,
                    onclick: move |_| {
                        let key = api_key_input().trim().to_string();
                        if key.starts_with("sk-") && key.len() >= 8 {
                            let config = Config {
                                api_key: key.clone(),
                                ..Config::default()
                            };
                            match save_config(&config) {
                                Ok(()) => on_complete.call(key),
                                Err(error) => validation_error.set(Some(format!("Failed to save: {}", error))),
                            }
                        } else {
                            validation_error.set(Some("Key must start with 'sk-' and be at least 8 characters.".to_string()));
                        }
                    },
                    if api_key_input().trim().len() >= 8 { "Get Started \u{2192}" } else { "Enter API key" }
                }

                div {
                    color: "#6b7280",
                    font_size: "11px",
                    text_align: "center",
                    "Don't have a key? Get one at platform.deepseek.com"
                }
            }

            div {
                display: "flex",
                gap: "12px",
                font_size: "12px",

                a {
                    color: "#3b82f6",
                    "platform.deepseek.com"
                }
            }
        }
    }
}
