use crate::mcp_state::McpUiState;
use deepseek_mobile_core::McpServerStatus;
use dioxus::prelude::*;

fn status_color(status: &McpServerStatus) -> &'static str {
    match status {
        McpServerStatus::Connected => "#16a34a",
        McpServerStatus::Connecting => "#ca8a04",
        McpServerStatus::Disconnected => "#6b7280",
        McpServerStatus::Error(_) => "#dc2626",
    }
}

fn status_label(status: &McpServerStatus) -> &'static str {
    status.label()
}

pub fn mcp_panel(mut state: Signal<McpUiState>) -> Element {
    let mut loaded = use_signal(|| false);
    if !*loaded.peek() {
        state.write().refresh();
        loaded.set(true);
    }

    let servers = state.read().registry.servers.clone();
    let error = state.read().last_error.clone();
    let connected = state.read().connected_count();
    let tool_count = state.read().tool_count();

    let server_cards: Vec<Element> = servers.iter().map(|s| {
        let server_name = s.config.name.clone();
        let server_name2 = server_name.clone();
        let desc = s.config.description.clone().unwrap_or_default();
        let transport_label = s.config.transport.label();
        let transport_kind = s.config.transport.kind_str();
        let status = s.status.clone();
        let color = status_color(&status);
        let slabel = status_label(&status);
        let enabled = s.config.enabled;
        let tool_count = s.tools.len();
        let error_detail = match &status {
            McpServerStatus::Error(msg) => Some(msg.clone()),
            _ => None,
        };

        rsx! {
            div {
                key: "{server_name}",
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
                        "{server_name}"
                    }
                    div {
                        display: "flex",
                        gap: "6px",
                        align_items: "center",

                        div {
                            background_color: color,
                            color: "white",
                            border_radius: "6px",
                            padding: "2px 8px",
                            font_size: "11px",
                            font_weight: "bold",
                            "{slabel}"
                        }
                        button {
                            background_color: if enabled { "#16a34a" } else { "#374151" },
                            border: "none",
                            border_radius: "8px",
                            padding: "4px 10px",
                            color: "white",
                            font_size: "11px",
                            onclick: move |_| {
                                state.write().toggle_server(&server_name, !enabled);
                            },
                            if enabled { "ON" } else { "OFF" }
                        }
                        button {
                            background_color: "#991b1b",
                            border: "none",
                            border_radius: "8px",
                            padding: "4px 10px",
                            color: "white",
                            font_size: "11px",
                            onclick: move |_| state.write().remove_server(&server_name2),
                            "Del"
                        }
                    }
                }

                div {
                    display: "flex",
                    gap: "8px",
                    font_size: "11px",
                    color: "#6b7280",
                    div { "{transport_kind}" }
                    div { "{transport_label}" }
                    if tool_count > 0 {
                        div { "{tool_count} tool(s)" }
                    }
                }

                if !desc.is_empty() {
                    div { color: "#9ca3af", font_size: "12px", "{desc}" }
                }

                if let Some(ref msg) = error_detail {
                    div { color: "#fca5a5", font_size: "12px", "{msg}" }
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

                div { font_size: "20px", font_weight: "bold", "MCP Servers ({servers.len()})" }
                div {
                    display: "flex",
                    gap: "8px",
                    if connected > 0 {
                        div { color: "#16a34a", font_size: "12px", "{connected} connected" }
                    }
                    if tool_count > 0 {
                        div { color: "#3b82f6", font_size: "12px", "{tool_count} tools" }
                    }
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

            if servers.is_empty() {
                div {
                    color: "#6b7280",
                    font_size: "13px",
                    text_align: "center",
                    padding: "16px 0",
                    "No MCP servers configured. Add servers via mcp.json in the data directory."
                }
            }

            div {
                display: "flex",
                flex_direction: "column",
                gap: "8px",
                {server_cards.into_iter()}
            }
        }
    }
}
