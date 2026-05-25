use crate::runtime_health::RuntimeHealthSnapshot;
use deepseek_mobile_core::config::ExecutionMode;
use dioxus::prelude::*;

pub fn health_panel(snapshot: RuntimeHealthSnapshot) -> Element {
    let mode_label = match snapshot.execution_mode {
        ExecutionMode::Plan => "Plan (tools disabled)",
        ExecutionMode::Agent => "Agent (recommended)",
        ExecutionMode::Yolo => "YOLO (auto-approve)",
    };

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            gap: "12px",
            color: "white",

            div {
                font_size: "18px",
                font_weight: "bold",
                "Runtime health"
            }
            div {
                color: "#9ca3af",
                font_size: "12px",
                "What works right now on this device. This is a coding agent — not full-phone or full-PC remote control."
            }

            {health_row("DeepSeek API", if snapshot.api_configured { "Configured" } else { "Missing" }, snapshot.api_configured)}
            {health_row("Execution mode", mode_label, snapshot.execution_mode == ExecutionMode::Agent)}
            {health_row("PC Host", &snapshot.pc_status_label, snapshot.pc_online)}
            {health_row(
                "PC workspace",
                if snapshot.pc_workspace_active { "Active" } else { "None — local phone sandbox only" },
                snapshot.pc_workspace_active,
            )}
            {health_row(
                "Termux workspace",
                if snapshot.termux_valid {
                    "Valid path saved"
                } else if snapshot.termux_configured {
                    "Path invalid"
                } else {
                    "Not configured"
                },
                snapshot.termux_valid,
            )}
            {health_row(
                "MCP servers",
                &format!("{}/{} connected", snapshot.mcp_servers_connected, snapshot.mcp_servers_total),
                snapshot.mcp_servers_connected > 0 || snapshot.mcp_servers_total == 0,
            )}
            {health_row(
                "Native bridge",
                if snapshot.native_pending {
                    "Waiting for Android callback"
                } else if snapshot.native_last_error.is_some() {
                    "Error (see below)"
                } else {
                    "Idle"
                },
                !snapshot.native_pending && snapshot.native_last_error.is_none(),
            )}

            if let Some(error) = snapshot.native_last_error {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "12px",
                    padding: "10px",
                    font_size: "12px",
                    "{error}"
                }
            }

            div {
                background_color: "#111827",
                border: "1px solid #374151",
                border_radius: "14px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "6px",

                div { font_weight: "bold", font_size: "13px", "PC gateway URLs" }
                for hint in snapshot.network_hints {
                    div { color: "#9ca3af", font_size: "11px", "{hint}" }
                }
            }

            if !snapshot.recommendations.is_empty() {
                div {
                    background_color: "#111827",
                    border: "1px solid #374151",
                    border_radius: "14px",
                    padding: "12px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "8px",

                    div { font_weight: "bold", font_size: "13px", "Next steps" }
                    for line in snapshot.recommendations {
                        div { color: "#d1d5db", font_size: "12px", "• {line}" }
                    }
                }
            }

            div {
                color: "#6b7280",
                font_size: "11px",
                "Offline Android setup: tools/android/DOWNLOAD_BUDGET.md"
            }
        }
    }
}

fn health_row(label: &str, value: &str, ok: bool) -> Element {
    let (bg, border) = if ok {
        ("#064e3b", "#10b981")
    } else {
        ("#1f2937", "#4b5563")
    };
    let border_style = format!("1px solid {border}");
    rsx! {
        div {
            background_color: bg,
            border: "{border_style}",
            border_radius: "12px",
            padding: "10px 12px",
            display: "flex",
            justify_content: "space-between",
            gap: "12px",
            font_size: "12px",
            span { font_weight: "bold", "{label}" }
            span { color: "#d1d5db", text_align: "right", "{value}" }
        }
    }
}
