use dioxus::prelude::*;
use deepseek_mobile_core::{DeepSeekCore, Config, Message};

fn main() {
    dioxus_mobile::launch(app);
}

fn app() -> Element {
    let mut messages = use_signal(Vec::<(String, String)>::new); // (role, content)
    let mut input = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    rsx! {
        div {
            background_color: "#0f0f0f",
            color: "white",
            height: "100vh",
            padding: "16px",
            display: "flex",
            flex_direction: "column",

            // Header
            div {
                font_size: "20px",
                font_weight: "bold",
                margin_bottom: "12px",
                "DeepSeek Mobile"
            }

            // Chat area
            div {
                flex: "1",
                background_color: "#1a1a1a",
                padding: "12px",
                border_radius: "8px",
                overflow_y: "auto",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                for (role, content) in messages() {
                    div {
                        background_color: if role == "user" { "#3b82f6" } else { "#333" },
                        padding: "10px 14px",
                        border_radius: "8px",
                        max_width: "85%",
                        align_self: if role == "user" { "flex-end" } else { "flex-start" },
                        "{content}"
                    }
                }

                if is_loading() {
                    div { "Thinking..." }
                }
            }

            // Input area
            div {
                display: "flex",
                gap: "8px",
                margin_top: "12px",

                input {
                    flex: "1",
                    background_color: "#222",
                    color: "white",
                    padding: "12px",
                    border: "1px solid #444",
                    border_radius: "8px",
                    placeholder: "Message DeepSeek...",
                    oninput: move |e| input.set(e.value()),
                }

                button {
                    background_color: "#3b82f6",
                    color: "white",
                    padding: "0 20px",
                    border_radius: "8px",
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
                            
                            // Add system message for better results
                            let mut chat_messages = vec![
                                Message { role: "system".to_string(), content: "You are a helpful coding assistant.".to_string() }
                            ];
                            
                            // Simple history (last few messages)
                            for (role, content) in messages() {
                                if role != "system" {
                                    chat_messages.push(Message { role, content });
                                }
                            }
                            
                            match core.process_with_messages(chat_messages).await {
                                Ok(response) => {
                                    messages.push(("assistant".to_string(), response));
                                }
                                Err(e) => {
                                    messages.push(("assistant".to_string(), format!("Error: {}", e)));
                                }
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