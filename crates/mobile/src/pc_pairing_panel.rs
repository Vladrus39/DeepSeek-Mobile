use crate::native_bridge::NativeBridgeState;
use crate::pc_pairing_state::{
    PcPairingUiState, PcPairingUiStatus, PcReconnectAction, PcReconnectEffect,
};
use dioxus::prelude::*;

pub fn pc_pairing_panel(mut state: Signal<PcPairingUiState>, mut native_bridge: Signal<NativeBridgeState>) -> Element {
    let snapshot = state();
    let status_text = snapshot.status_text();
    let action_label = snapshot.primary_action_label();
    let zip_path = snapshot
        .export
        .as_ref()
        .map(|export| export.zip_path.display().to_string())
        .unwrap_or_else(|| "Pairing ZIP has not been created yet".to_string());
    let status_badge = status_badge_text(&snapshot.status);
    let active_route = snapshot.active_route_text();
    let endpoint_rows = snapshot.endpoint_health_rows();
    let discovery_rows = snapshot.discovery_rows();
    let reconnect_controls = snapshot.reconnect_controls();

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
                justify_content: "space-between",
                align_items: "center",
                gap: "12px",

                div {
                    div {
                        font_size: "18px",
                        font_weight: "bold",
                        "Connect PC Host"
                    }
                    div {
                        color: "#9ca3af",
                        font_size: "13px",
                        "Create a one-click pairing ZIP, open it on the computer, then control the PC workspace from Android."
                    }
                }

                div {
                    background_color: status_badge_color(&snapshot.status),
                    color: "white",
                    padding: "6px 10px",
                    border_radius: "999px",
                    font_size: "12px",
                    font_weight: "bold",
                    "{status_badge}"
                }
            }

            div {
                background_color: "#1f2937",
                border_radius: "12px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "6px",

                div { color: "#d1d5db", font_size: "13px", "Status" }
                div { white_space: "pre-wrap", "{status_text}" }
            }

            div {
                background_color: "#1f2937",
                border_radius: "12px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "6px",

                div { color: "#d1d5db", font_size: "13px", "Active route" }
                div {
                    color: "#e5e7eb",
                    font_size: "13px",
                    white_space: "pre-wrap",
                    "{active_route}"
                }
            }

            div {
                background_color: "#1f2937",
                border_radius: "12px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div { color: "#d1d5db", font_size: "13px", "Reconnect controls" }
                for control in reconnect_controls {
                    button {
                        background_color: if control.enabled { "#2563eb" } else { "#374151" },
                        color: "white",
                        padding: "10px 12px",
                        border_radius: "10px",
                        border: "1px solid #4b5563",
                        text_align: "left",
                        disabled: !control.enabled,
                        onclick: move |_| {
                            let action = control.action.clone();
                            let mut next_state = state();
                            let effect = next_state.apply_reconnect_action(action.clone());
                            state.set(next_state);

                            match effect {
                                PcReconnectEffect::StartDiscovery { request_id } => {
                                    let mut bridge = native_bridge();
                                    bridge.enqueue_pc_gateway_discovery(request_id);
                                    native_bridge.set(bridge);
                                }
                                PcReconnectEffect::RetryRoute { .. }
                                | PcReconnectEffect::SelectedRoute { .. }
                                | PcReconnectEffect::None => {}
                            }
                        },
                        div { font_weight: "bold", "{control.label}" }
                        div { color: "#d1d5db", font_size: "12px", "{control.description}" }
                    }
                }
            }

            div {
                background_color: "#1f2937",
                border_radius: "12px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div { color: "#d1d5db", font_size: "13px", "Discovery candidates" }
                for row in discovery_rows {
                    div {
                        color: "#e5e7eb",
                        font_size: "12px",
                        white_space: "pre-wrap",
                        border_top: "1px solid #374151",
                        padding_top: "8px",
                        "{row}"
                    }
                }
            }

            div {
                background_color: "#1f2937",
                border_radius: "12px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div { color: "#d1d5db", font_size: "13px", "Endpoint health" }
                for row in endpoint_rows {
                    div {
                        color: "#e5e7eb",
                        font_size: "12px",
                        white_space: "pre-wrap",
                        border_top: "1px solid #374151",
                        padding_top: "8px",
                        "{row}"
                    }
                }
            }

            div {
                background_color: "#1f2937",
                border_radius: "12px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "6px",

                div { color: "#d1d5db", font_size: "13px", "Pairing file" }
                div {
                    color: "#e5e7eb",
                    font_size: "13px",
                    white_space: "pre-wrap",
                    "{zip_path}"
                }
            }

            div {
                display: "flex",
                gap: "8px",
                flex_wrap: "wrap",

                button {
                    background_color: "#2563eb",
                    color: "white",
                    padding: "10px 14px",
                    border_radius: "10px",
                    border: "none",
                    font_weight: "bold",
                    "{action_label}"
                }

                button {
                    background_color: "#374151",
                    color: "white",
                    padding: "10px 14px",
                    border_radius: "10px",
                    border: "none",
                    "Instructions"
                }
            }
        }
    }
}

fn status_badge_text(status: &PcPairingUiStatus) -> &'static str {
    match status {
        PcPairingUiStatus::NotConfigured => "SETUP",
        PcPairingUiStatus::ReadyToExport => "READY",
        PcPairingUiStatus::Exported => "ZIP READY",
        PcPairingUiStatus::WaitingForPc => "WAITING",
        PcPairingUiStatus::Online => "ONLINE",
        PcPairingUiStatus::Offline => "OFFLINE",
        PcPairingUiStatus::Error(_) => "ERROR",
    }
}

fn status_badge_color(status: &PcPairingUiStatus) -> &'static str {
    match status {
        PcPairingUiStatus::Online => "#059669",
        PcPairingUiStatus::WaitingForPc | PcPairingUiStatus::ReadyToExport | PcPairingUiStatus::Exported => "#2563eb",
        PcPairingUiStatus::Error(_) => "#dc2626",
        PcPairingUiStatus::Offline => "#7f1d1d",
        PcPairingUiStatus::NotConfigured => "#4b5563",
    }
}

#[cfg(test)]
mod tests {
    use super::{status_badge_text, PcPairingUiStatus};

    #[test]
    fn status_badges_match_pairing_state() {
        assert_eq!(status_badge_text(&PcPairingUiStatus::NotConfigured), "SETUP");
        assert_eq!(status_badge_text(&PcPairingUiStatus::ReadyToExport), "READY");
        assert_eq!(status_badge_text(&PcPairingUiStatus::Exported), "ZIP READY");
        assert_eq!(status_badge_text(&PcPairingUiStatus::WaitingForPc), "WAITING");
        assert_eq!(status_badge_text(&PcPairingUiStatus::Online), "ONLINE");
        assert_eq!(status_badge_text(&PcPairingUiStatus::Offline), "OFFLINE");
        assert_eq!(status_badge_text(&PcPairingUiStatus::Error("x".to_string())), "ERROR");
    }

    #[test]
    fn reconnect_action_type_is_imported() {
        assert_eq!(format!("{:?}", PcReconnectAction::ScanAgain), "ScanAgain");
    }
}