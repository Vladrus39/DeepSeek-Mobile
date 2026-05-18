//! Terminal session panel for the mobile cockpit.
use crate::terminal_state::TerminalUiState;
use dioxus::prelude::*;

pub fn terminal_panel(
    mut state: Signal<TerminalUiState>,
    on_open_terminal: EventHandler<()>,
    on_send_input: EventHandler<String>,
    on_close_terminal: EventHandler<String>,
) -> Element {
    let active_count = state.read().active_session_count();
    let selected_id = state.read().selected_session_id.clone();
    let input_text = state.read().input_text.clone();
    let loading = state.read().loading;
    let error = state.read().error.clone();
    let output_lines: Vec<String> = state.read().selected_output().iter().map(|l| l.to_string()).collect();
    // Collect session open states into a snapshot to avoid temporary lifetime issues
    let session_open_states: std::collections::HashMap<String, bool> = state.read().sessions.iter()
        .map(|s| (s.id.clone(), s.is_open))
        .collect();
    let selected_is_active = selected_id.as_ref()
        .and_then(|id| session_open_states.get(id))
        .copied()
        .unwrap_or(false);

    // Snapshot session data to avoid temporary lifetime issues
    let sessions_snapshot: Vec<(String, bool, bool, String)> = state.read().sessions.iter().map(|s| {
        (s.id.clone(), s.is_open, selected_id.as_deref() == Some(&s.id), s.title.clone())
    }).collect();

    let session_tabs: Vec<Element> = sessions_snapshot.iter().map(|(sid, is_active, is_selected, title)| {
        let sid0 = sid.clone();
        let sid1 = sid.clone();
        let label = if title.len() > 20 { format!("{}…", &title[..20]) } else { title.clone() };

        rsx! {
            div {
                key: "{sid0}",
                background_color: if *is_selected { "#1e3a8a" } else { "#1f2937" },
                border: if *is_selected { "1px solid #3b82f6" } else { "1px solid #374151" },
                border_radius: "8px", padding: "6px 10px", font_size: "12px",
                display: "flex", gap: "6px", align_items: "center",
                span { color: if *is_active { "#10b981" } else { "#6b7280" }, font_size: "10px", "●" }
                button {
                    background_color: "transparent", border: "none", color: "white",
                    padding: "0", font_size: "12px",
                    onclick: move |_| { let mut s = state.write(); s.select_session(&sid0); },
                    "{label}"
                }
                button {
                    background_color: "transparent", border: "none", color: "#ef4444",
                    font_size: "11px", padding: "0 0 0 4px",
                    onclick: move |_| on_close_terminal.call(sid1.clone()),
                    "✕"
                }
            }
        }
    }).collect();

    rsx! {
        div {
            background_color: "#111827", color: "white", border: "1px solid #374151",
            border_radius: "16px", padding: "12px",
            display: "flex", flex_direction: "column", gap: "12px",

            div {
                background_color: "#0f172a", border: "1px solid #334155",
                border_radius: "14px", padding: "12px",
                display: "flex", justify_content: "space_between", align_items: "center",
                div { font_size: "18px", font_weight: "bold", "Terminal ({active_count} active)" }
                button {
                    background_color: "#2563eb", border: "none", border_radius: "10px",
                    padding: "6px 14px", color: "white", font_size: "13px",
                    onclick: move |_| on_open_terminal.call(()),
                    disabled: if loading { "true" } else { "false" }, "+ New Session"
                }
            }

            if !session_tabs.is_empty() {
                div { display: "flex", gap: "6px", flex_wrap: "wrap", {session_tabs.into_iter()} }
            }

            div {
                background_color: "#020617", border: "1px solid #1f2937",
                border_radius: "12px", padding: "10px",
                min_height: "150px", max_height: "300px", overflow_y: "auto",
                font_family: "monospace", font_size: "12px",
                line_height: "1.5", white_space: "pre_wrap",
                if output_lines.is_empty() {
                    div { color: "#6b7280", font_size: "13px", "Terminal output will appear here." }
                } else {
                    for line in &output_lines {
                        div { key: "{line}", color: "#d1d5db", "{line}" }
                    }
                    if selected_is_active {
                        div { color: "#10b981", font_size: "11px", "─ session active ─" }
                    }
                }
            }

            if selected_is_active {
                div { display: "flex", gap: "8px", align_items: "center",
                    input {
                        flex: "1", background_color: "#1f2937", color: "white",
                        padding: "10px", border: "1px solid #4b5563",
                        border_radius: "10px", font_size: "13px", font_family: "monospace",
                        placeholder: "Enter command or input...",
                        value: "{input_text}",
                        oninput: move |e| { state.write().input_text = e.value(); },
                    }
                    button {
                        background_color: "#2563eb", border: "none",
                        border_radius: "10px", padding: "8px 16px",
                        color: "white", font_size: "13px",
                        disabled: if input_text.trim().is_empty() || loading { "true" } else { "false" },
                        onclick: {
                            let text = input_text.clone();
                            move |_| { let t = text.trim().to_string(); if !t.is_empty() { on_send_input.call(t); state.write().input_text = String::new(); } }
                        },
                        "Send"
                    }
                }
            }

            if let Some(ref err) = error {
                div { background_color: "#7f1d1d", border: "1px solid #dc2626",
                    border_radius: "12px", padding: "10px", color: "white", font_size: "12px", "{err}" }
            }
        }
    }
}
