use dioxus::prelude::*;
use deepseek_mobile_core::{DeepSeekCore, Config};

fn main() {
    dioxus_mobile::launch(app);
}

fn app() -> Element {
    let mut input = use_signal(String::new);
    let mut output = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    rsx! {
        div {
            background_color: "#0f0f0f",
            color: "white",
            height: "100vh",
            padding: "16px",
            display: "flex",
            flex_direction: "column",

            h1 { "DeepSeek Mobile" }

            div {
                flex: "1",
                background_color: "#1a1a1a",
                padding: "12px",
                border_radius: "8px",
                overflow_y: "auto",
                white_space: "pre-wrap",
                "{output}"
            }

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
                    oninput: move |e| input.set(e.value()),
                }

                button {
                    background_color: "#3b82f6",
                    color: "white",
                    padding: "12px 24px",
                    border_radius: "8px",
                    onclick: move |_| {
                        let prompt = input();
                        if !prompt.is_empty() {
                            is_loading.set(true);
                            spawn(async move {
                                let config = Config::default();
                                let core = DeepSeekCore::new(config);
                                match core.process(prompt).await {
                                    Ok(response) => output.set(response),
                                    Err(e) => output.set(format!("Error: {}", e)),
                                }
                                is_loading.set(false);
                            });
                        }
                    },
                    "Send"
                }
            }

            if is_loading() {
                div { "Thinking..." }
            }
        }
    }
}