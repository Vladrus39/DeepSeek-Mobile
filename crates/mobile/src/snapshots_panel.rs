use crate::snapshots_state::SnapshotsUiState;
use dioxus::prelude::*;

pub fn snapshots_panel(state: &SnapshotsUiState) -> Element {
    let latest = state.latest();

    rsx! {
        div {
            background_color: "#111827",
            color: "white",
            border: "1px solid #374151",
            border_radius: "16px",
            padding: "12px",
            display: "flex",
            flex_direction: "column",
            gap: "12px",

            div {
                background_color: "#0f172a",
                border: "1px solid #334155",
                border_radius: "14px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "6px",

                div { font_size: "18px", font_weight: "bold", "Snapshots" }
                if let Some(snapshot) = latest {
                    div { color: "#d1d5db", font_size: "13px", "Latest safety point: {snapshot.id}" }
                    div { color: "#9ca3af", font_size: "12px", "{snapshot.file_count} file(s) · {snapshot.total_bytes} bytes · {snapshot.reason}" }
                } else {
                    div { color: "#9ca3af", font_size: "13px", "No snapshots have been surfaced yet." }
                }
            }

            if let Some(error) = state.last_error.as_ref() {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "12px",
                    padding: "10px",
                    color: "white",
                    font_size: "12px",
                    "{error}"
                }
            }

            div {
                background_color: "#020617",
                border: "1px solid #1f2937",
                border_radius: "14px",
                padding: "10px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div { font_size: "14px", font_weight: "bold", "Available snapshots" }
                if state.snapshots.is_empty() {
                    div { color: "#9ca3af", font_size: "12px", "Approved destructive tools will create safety snapshots automatically." }
                } else {
                    for snapshot in state.snapshots.iter().take(12) {
                        div {
                            background_color: "#111827",
                            border: "1px solid #1f2937",
                            border_radius: "12px",
                            padding: "10px",
                            display: "flex",
                            flex_direction: "column",
                            gap: "4px",

                            div { font_size: "13px", font_weight: "bold", "{snapshot.id}" }
                            div { color: "#9ca3af", font_size: "12px", "{snapshot.file_count} file(s) · {snapshot.total_bytes} bytes" }
                            div { color: "#d1d5db", font_size: "12px", "{snapshot.reason}" }
                        }
                    }
                }
            }

            div {
                background_color: "#422006",
                border: "1px solid #ca8a04",
                border_radius: "14px",
                padding: "10px",
                color: "#fde68a",
                font_size: "12px",
                "Restore actions are intentionally not wired into this panel yet; the next rollback step is a dedicated confirmation screen with deletion warnings."
            }
        }
    }
}
