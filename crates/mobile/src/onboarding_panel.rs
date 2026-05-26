use crate::dev_bootstrap::prefill_api_key_for_onboarding;
use crate::settings_state::save_config;
use crate::termux_state::TermuxWorkspaceState;
use deepseek_mobile_core::config::{Config, ExecutionMode};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum OnboardingStep {
    Api,
    Workspaces,
    Done,
}

/// Full-screen setup wizard: API → Termux (full agent) → optional PC note → start.
pub fn onboarding_panel(on_complete: EventHandler<String>) -> Element {
    let mut step = use_signal(|| OnboardingStep::Api);
    let mut api_key_input = use_signal(prefill_api_key_for_onboarding);
    let mut termux_path_input =
        use_signal(|| "/data/data/com.termux/files/home/project".to_string());
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
            gap: "16px",

            div {
                font_size: "28px",
                font_weight: "bold",
                "DeepSeek Mobile"
            }
            div {
                color: "#9ca3af",
                font_size: "13px",
                text_align: "center",
                max_width: "360px",
                match step() {
                    OnboardingStep::Api => "Step 1/3 — Connect the coding agent API.",
                    OnboardingStep::Workspaces => "Step 2/3 — Termux = full agent on phone (recommended). PC Host is optional later.",
                    OnboardingStep::Done => "Step 3/3 — Ready to open the cockpit.",
                }
            }

            div {
                background_color: "#111827",
                border: "1px solid #374151",
                border_radius: "18px",
                padding: "20px",
                width: "100%",
                max_width: "400px",
                display: "flex",
                flex_direction: "column",
                gap: "14px",

                match step() {
                    OnboardingStep::Api => rsx! {
                        div { font_weight: "bold", "DeepSeek API key" }
                        div {
                            color: "#9ca3af",
                            font_size: "12px",
                            "Required for chat. Stored locally on this device only."
                        }
                        input {
                            background_color: "#1f2937",
                            color: "white",
                            padding: "14px",
                            border: if validation_error().is_some() { "1px solid #ef4444" } else { "1px solid #4b5563" },
                            border_radius: "14px",
                            placeholder: "sk-...",
                            value: "{api_key_input}",
                            oninput: move |e| {
                                api_key_input.set(e.value());
                                validation_error.set(None);
                            },
                        }
                        if let Some(error) = validation_error() {
                            div { color: "#fca5a5", font_size: "12px", "{error}" }
                        }
                        button {
                            background_color: "#3b82f6",
                            color: "white",
                            padding: "14px",
                            border_radius: "14px",
                            border: "none",
                            font_weight: "bold",
                            disabled: api_key_input().trim().len() < 8,
                            onclick: move |_| {
                                let key = api_key_input().trim().to_string();
                                if key.starts_with("sk-") {
                                    step.set(OnboardingStep::Workspaces);
                                } else {
                                    validation_error.set(Some("Key must start with sk-.".to_string()));
                                }
                            },
                            "Continue"
                        }
                    },
                    OnboardingStep::Workspaces => rsx! {
                        div { font_weight: "bold", "Full agent on this phone" }
                        {capability_card("Termux project (main)", "Install Termux, allow-external-apps in ~/.termux/termux.properties, grant RUN_COMMAND. Same role as terminal on desktop TUI.", true)}
                        div {
                            color: "#9ca3af",
                            font_size: "11px",
                            "Termux project path (absolute):"
                        }
                        input {
                            background_color: "#1f2937",
                            color: "white",
                            padding: "12px",
                            border: "1px solid #4b5563",
                            border_radius: "12px",
                            value: "{termux_path_input}",
                            oninput: move |e| termux_path_input.set(e.value()),
                        }
                        {capability_card("App sandbox (lite)", "Small edits and ZIP import without Termux. No shell here.", false)}
                        {capability_card("PC Host (optional)", "Later: pair PC only if the repo is too large for the phone. Not required.", false)}
                        button {
                            background_color: "#3b82f6",
                            color: "white",
                            padding: "14px",
                            border_radius: "14px",
                            border: "none",
                            font_weight: "bold",
                            onclick: move |_| step.set(OnboardingStep::Done),
                            "Continue"
                        }
                        button {
                            background_color: "transparent",
                            color: "#9ca3af",
                            padding: "10px",
                            border: "1px solid #4b5563",
                            border_radius: "12px",
                            onclick: move |_| {
                                termux_path_input.set(String::new());
                                step.set(OnboardingStep::Done);
                            },
                            "Skip Termux for now (sandbox only)"
                        }
                    },
                    OnboardingStep::Done => rsx! {
                        div { font_weight: "bold", "You're set" }
                        div {
                            color: "#9ca3af",
                            font_size: "12px",
                            "Open Chat with Agent mode, Health to confirm Termux, Settings to adjust path. PC Host panel only if you need desktop boost."
                        }
                        button {
                            background_color: "#10b981",
                            color: "white",
                            padding: "14px",
                            border_radius: "14px",
                            border: "none",
                            font_weight: "bold",
                            onclick: move |_| {
                                let key = api_key_input().trim().to_string();
                                let config = Config {
                                    api_key: key.clone(),
                                    execution_mode: ExecutionMode::Agent,
                                    ..Config::default()
                                };
                                match save_config(&config) {
                                    Ok(()) => {
                                        let path = termux_path_input().trim().to_string();
                                        if !path.is_empty() {
                                            let mut termux = TermuxWorkspaceState::default();
                                            termux.set_path(path);
                                            termux.set_label("Termux Project");
                                            if termux.is_valid() {
                                                let _ = termux.save_to_base_dir(crate::mobile_runtime_config::default_data_dir());
                                            }
                                        }
                                        on_complete.call(key);
                                    }
                                    Err(error) => validation_error.set(Some(format!("Failed to save: {}", error))),
                                }
                            },
                            "Open cockpit"
                        }
                    },
                }
            }
        }
    }
}

fn capability_card(title: &str, body: &str, available_now: bool) -> Element {
    let border = if available_now { "#10b981" } else { "#4b5563" };
    let border_style = format!("1px solid {border}");
    rsx! {
        div {
            border: "{border_style}",
            border_radius: "12px",
            padding: "10px",
            background_color: "#1f2937",
            div { font_weight: "bold", font_size: "13px", "{title}" }
            div { color: "#9ca3af", font_size: "11px", margin_top: "4px", "{body}" }
        }
    }
}
