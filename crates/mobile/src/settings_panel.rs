use crate::settings_state::{save_config, SettingsFormState};
use crate::termux_state::TermuxWorkspaceState;
use deepseek_mobile_core::config::{ExecutionMode, ExternalAccessMode, ModelMode, ThinkingLevel};
use dioxus::prelude::*;

pub fn settings_panel(
    mut state: Signal<SettingsFormState>,
    mut termux_state: Signal<TermuxWorkspaceState>,
) -> Element {
    let s = state();

    rsx! {
        div {
            background_color: "#111827",
            color: "white",
            border: "1px solid #374151",
            border_radius: "16px",
            padding: "16px",
            display: "flex",
            flex_direction: "column",
            gap: "14px",

            div {
                font_size: "18px",
                font_weight: "bold",
                "Settings"
            }

            // ── DeepSeek API ──
            SectionCard { title: "DeepSeek API" }
            LabeledField {
                label: "API Key",
                help: "Your DeepSeek API key (starts with sk-…).",
                value: s.api_key.clone(),
                password: true,
                oninput: move |v: String| {
                    let mut next = state();
                    next.api_key = v;
                    state.set(next);
                },
            }

            // ── Model & Execution ──
            SectionCard { title: "Model & Execution" }

            SelectField {
                label: "Model",
                value: model_mode_key(&s.model_mode),
                options: vec![
                    ("auto".to_string(), "Auto (Flash for simple, Pro for complex)".to_string()),
                    ("flash".to_string(), "V4 Flash (fast, cheap)".to_string()),
                    ("pro".to_string(), "V4 Pro (deep reasoning)".to_string()),
                ],
                onchange: move |v: String| {
                    let mut next = state();
                    next.model_mode = match v.as_str() {
                        "flash" => ModelMode::Flash,
                        "pro" => ModelMode::Pro,
                        _ => ModelMode::Auto,
                    };
                    state.set(next);
                },
            }

            SelectField {
                label: "Execution mode",
                value: execution_mode_key(&s.execution_mode),
                options: vec![
                    ("agent".to_string(), "Agent — propose + ask approval".to_string()),
                    ("plan".to_string(), "Plan — think only, no tools".to_string()),
                    ("yolo".to_string(), "YOLO — auto-approve safe tools".to_string()),
                ],
                onchange: move |v: String| {
                    let mut next = state();
                    next.execution_mode = match v.as_str() {
                        "plan" => ExecutionMode::Plan,
                        "yolo" => ExecutionMode::Yolo,
                        _ => ExecutionMode::Agent,
                    };
                    state.set(next);
                },
            }

            SelectField {
                label: "Thinking level",
                value: thinking_level_key(&s.thinking_level),
                options: vec![
                    ("off".to_string(), "Off".to_string()),
                    ("low".to_string(), "Low".to_string()),
                    ("medium".to_string(), "Medium".to_string()),
                    ("high".to_string(), "High".to_string()),
                    ("max".to_string(), "Max".to_string()),
                ],
                onchange: move |v: String| {
                    let mut next = state();
                    next.thinking_level = match v.as_str() {
                        "off" => ThinkingLevel::Off,
                        "low" => ThinkingLevel::Low,
                        "medium" => ThinkingLevel::Medium,
                        "max" => ThinkingLevel::Max,
                        _ => ThinkingLevel::High,
                    };
                    state.set(next);
                },
            }

            SelectField {
                label: "External access",
                value: external_access_key(&s.external_access),
                options: vec![
                    ("workspace_only".to_string(), "Workspace only".to_string()),
                    ("ask_every_time".to_string(), "Ask every time".to_string()),
                    ("allowed_by_grant".to_string(), "Allowed by user grant".to_string()),
                ],
                onchange: move |v: String| {
                    let mut next = state();
                    next.external_access = match v.as_str() {
                        "ask_every_time" => ExternalAccessMode::AskEveryTime,
                        "allowed_by_grant" => ExternalAccessMode::AllowedByUserGrant,
                        _ => ExternalAccessMode::WorkspaceOnly,
                    };
                    state.set(next);
                },
            }

            if matches!(s.external_access, ExternalAccessMode::AllowedByUserGrant) {
                LabeledField {
                    label: "Trusted paths (one per line)",
                    help: "Absolute paths outside the workspace the agent may read/write when grant mode is on (PC paths when paired, phone paths in sandbox).",
                    value: s.trusted_external_paths.clone(),
                    password: false,
                    oninput: move |v: String| {
                        let mut next = state();
                        next.trusted_external_paths = v;
                        state.set(next);
                    },
                }
            }

            // ── GitHub Integration ──
            SectionCard { title: "GitHub Integration" }

            LabeledField {
                label: "GitHub Token",
                help: "Personal access token (repo scope). Leave empty to skip GitHub features.",
                value: s.github_token.clone(),
                password: true,
                oninput: move |v: String| {
                    let mut next = state();
                    next.github_token = v;
                    state.set(next);
                },
            }

            LabeledField {
                label: "Repository",
                help: "e.g. Vladrus39/DeepSeek-Mobile",
                value: s.github_repo.clone(),
                password: false,
                oninput: move |v: String| {
                    let mut next = state();
                    next.github_repo = v;
                    state.set(next);
                },
            }

            LabeledField {
                label: "Default branch",
                help: "e.g. main",
                value: s.github_branch.clone(),
                password: false,
                oninput: move |v: String| {
                    let mut next = state();
                    next.github_branch = v;
                    state.set(next);
                },
            }

            ToggleField {
                label: "Auto commit & push after turn",
                checked: s.auto_commit_push,
                onchange: move |v: bool| {
                    let mut next = state();
                    next.auto_commit_push = v;
                    state.set(next);
                },
            }

            // ── Termux workspace ──
            SectionCard { title: "Termux Workspace" }

            {
                let tws = termux_state.read();
                let tws_path = tws.workspace_path.clone();
                let tws_label = tws.label.clone();
                let tws_err = tws.validation_error.clone();
                let tws_saved = tws.saved;
                let tws_is_valid = tws.is_valid();

                rsx! {
                    LabeledField {
                        label: "Workspace path",
                        help: "Absolute path in Termux, e.g. /data/data/com.termux/files/home/project",
                        value: tws_path.clone(),
                        password: false,
                        oninput: move |v: String| {
                            termux_state.write().set_path(v);
                        },
                    }
                    LabeledField {
                        label: "Label (optional)",
                        help: "Human-readable name for this workspace",
                        value: tws_label.clone(),
                        password: false,
                        oninput: move |v: String| {
                            termux_state.write().set_label(v);
                        },
                    }

                    if let Some(ref err) = tws_err {
                        div {
                            background_color: "#7f1d1d",
                            border: "1px solid #dc2626",
                            border_radius: "8px",
                            padding: "6px 10px",
                            font_size: "12px",
                            color: "#fca5a5",
                            "{err}"
                        }
                    }

                    if tws_saved {
                        div {
                            font_size: "11px",
                            color: "#6b7280",
                            "Termux config saved."
                        }
                    }

                    button {
                        background_color: if tws_is_valid { "#059669" } else { "#374151" },
                        color: "white",
                        border: if tws_is_valid { "1px solid #10b981" } else { "1px solid #4b5563" },
                        border_radius: "8px",
                        padding: "6px 12px",
                        font_size: "13px",
                        font_weight: "bold",
                        margin_top: "4px",
                        disabled: !tws_is_valid,
                        onclick: move |_| {
                            termux_state.write().save();
                        },
                        if tws_is_valid { "Save and activate Termux workspace" } else { "Enter Termux workspace path" }
                    }
                }
            }

            // ── Toast ──
            if s.saved {
                div {
                    background_color: "#065f46",
                    border: "1px solid #10b981",
                    border_radius: "12px",
                    padding: "10px",
                    font_size: "14px",
                    color: "white",
                    "Settings saved to local storage."
                }
            }
            if let Some(error) = &s.save_error {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "12px",
                    padding: "10px",
                    font_size: "14px",
                    color: "white",
                    "{error}"
                }
            }

            // ── Save button ──
            button {
                background_color: "#2563eb",
                color: "white",
                border: "1px solid #3b82f6",
                border_radius: "12px",
                padding: "12px",
                font_size: "15px",
                font_weight: "bold",
                onclick: move |_| {
                    let mut next = state();
                    match save_config(&next.to_config()) {
                        Ok(()) => {
                            next.saved = true;
                            next.save_error = None;
                        }
                        Err(e) => {
                            next.save_error = Some(e);
                        }
                    }
                    state.set(next);
                },
                "Save settings"
            }
        }
    }
}

// ── sub-components ──

#[component]
fn SectionCard(title: String) -> Element {
    rsx! {
        div {
            background_color: "#1f2937",
            border: "1px solid #374151",
            border_radius: "10px",
            padding: "8px 12px",
            font_size: "14px",
            font_weight: "bold",
            color: "#d1d5db",
            "{title}"
        }
    }
}

#[component]
fn LabeledField(
    label: String,
    help: String,
    value: String,
    password: bool,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            gap: "4px",

            div {
                font_size: "13px",
                color: "#d1d5db",
                "{label}"
            }
            input {
                r#type: if password { "password" } else { "text" },
                background_color: "#0d1117",
                color: "white",
                border: "1px solid #30363d",
                border_radius: "8px",
                padding: "10px",
                font_size: "14px",
                value: "{value}",
                oninput: move |evt: FormEvent| oninput.call(evt.value()),
            }
            div {
                font_size: "11px",
                color: "#6b7280",
                "{help}"
            }
        }
    }
}

#[component]
fn SelectField(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    onchange: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            gap: "4px",

            div {
                font_size: "13px",
                color: "#d1d5db",
                "{label}"
            }
            select {
                background_color: "#0d1117",
                color: "white",
                border: "1px solid #30363d",
                border_radius: "8px",
                padding: "10px",
                font_size: "14px",
                onchange: move |evt: FormEvent| onchange.call(evt.value()),
                for (key, label_text) in &options {
                    option {
                        value: "{key}",
                        selected: key == &value,
                        "{label_text}"
                    }
                }
            }
        }
    }
}

#[component]
fn ToggleField(label: String, checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        div {
            display: "flex",
            align_items: "center",
            justify_content: "space-between",
            gap: "8px",
            padding: "10px 0",

            div {
                font_size: "13px",
                color: "#d1d5db",
                "{label}"
            }
            label {
                display: "flex",
                align_items: "center",
                gap: "6px",
                cursor: "pointer",

                input {
                    r#type: "checkbox",
                    checked,
                    onchange: move |evt: FormEvent| {
                        let val = evt.value() == "true" || evt.value() == "on";
                        onchange.call(val);
                    },
                }
                span {
                    font_size: "12px",
                    color: if checked { "#10b981" } else { "#6b7280" },
                    if checked { "ON" } else { "OFF" }
                }
            }
        }
    }
}

// ── key helpers ──

fn model_mode_key(mode: &ModelMode) -> String {
    match mode {
        ModelMode::Auto => "auto".to_string(),
        ModelMode::Flash => "flash".to_string(),
        ModelMode::Pro => "pro".to_string(),
    }
}

fn execution_mode_key(mode: &ExecutionMode) -> String {
    match mode {
        ExecutionMode::Agent => "agent".to_string(),
        ExecutionMode::Plan => "plan".to_string(),
        ExecutionMode::Yolo => "yolo".to_string(),
    }
}

fn thinking_level_key(level: &ThinkingLevel) -> String {
    match level {
        ThinkingLevel::Off => "off".to_string(),
        ThinkingLevel::Low => "low".to_string(),
        ThinkingLevel::Medium => "medium".to_string(),
        ThinkingLevel::High => "high".to_string(),
        ThinkingLevel::Max => "max".to_string(),
    }
}

fn external_access_key(mode: &ExternalAccessMode) -> String {
    match mode {
        ExternalAccessMode::WorkspaceOnly => "workspace_only".to_string(),
        ExternalAccessMode::AskEveryTime => "ask_every_time".to_string(),
        ExternalAccessMode::AllowedByUserGrant => "allowed_by_grant".to_string(),
    }
}

