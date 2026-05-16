use crate::diagnostics_state::DiagnosticsUiState;
use deepseek_mobile_core::PcDiagnosticSeverity;
use dioxus::prelude::*;

pub fn diagnostics_panel(state: &DiagnosticsUiState) -> Element {
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
                gap: "8px",

                div { font_size: "18px", font_weight: "bold", "Diagnostics" }
                if state.has_data() {
                    div {
                        display: "flex",
                        gap: "8px",
                        flex_wrap: "wrap",
                        {stat_badge("Errors", state.error_count().to_string(), "#7f1d1d")}
                        {stat_badge("Warnings", state.warning_count().to_string(), "#78350f")}
                        {stat_badge("Items", state.diagnostics.len().to_string(), "#1f2937")}
                    }
                    if let Some(summary) = state.summary.as_ref() {
                        div { color: "#d1d5db", font_size: "13px", white_space: "pre-wrap", "{summary}" }
                    }
                    if let Some(path) = state.path.as_ref() {
                        div { color: "#93c5fd", font_size: "12px", "Path: {path}" }
                    }
                } else {
                    div { color: "#9ca3af", font_size: "13px", "No post-edit diagnostics have been surfaced yet." }
                }
            }

            if let Some(error) = state.error.as_ref() {
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

                div { font_size: "14px", font_weight: "bold", "Latest report" }
                if state.diagnostics.is_empty() {
                    div { color: "#9ca3af", font_size: "12px", "A clean report or missing provider will be shown here after edits." }
                } else {
                    for diagnostic in state.diagnostics.iter().take(20) {
                        div {
                            background_color: "#111827",
                            border: "1px solid #1f2937",
                            border_radius: "12px",
                            padding: "10px",
                            display: "flex",
                            flex_direction: "column",
                            gap: "4px",

                            div {
                                color: severity_color(&diagnostic.severity),
                                font_size: "12px",
                                font_weight: "bold",
                                "{severity_label(&diagnostic.severity)} · {diagnostic.path}:{diagnostic.line}:{diagnostic.column}"
                            }
                            div { color: "#d1d5db", font_size: "12px", white_space: "pre-wrap", "{diagnostic.message}" }
                            if let Some(source) = diagnostic.source.as_ref() {
                                div { color: "#6b7280", font_size: "11px", "{source}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn stat_badge(label: &'static str, value: String, background: &'static str) -> Element {
    rsx! {
        div {
            background_color: background,
            border: "1px solid #374151",
            border_radius: "999px",
            padding: "5px 9px",
            font_size: "12px",
            "{label}: {value}"
        }
    }
}

fn severity_label(severity: &PcDiagnosticSeverity) -> &'static str {
    match severity {
        PcDiagnosticSeverity::Error => "ERROR",
        PcDiagnosticSeverity::Warning => "WARNING",
        PcDiagnosticSeverity::Info => "INFO",
        PcDiagnosticSeverity::Hint => "HINT",
    }
}

fn severity_color(severity: &PcDiagnosticSeverity) -> &'static str {
    match severity {
        PcDiagnosticSeverity::Error => "#fca5a5",
        PcDiagnosticSeverity::Warning => "#fcd34d",
        PcDiagnosticSeverity::Info => "#93c5fd",
        PcDiagnosticSeverity::Hint => "#c4b5fd",
    }
}

#[cfg(test)]
mod tests {
    use super::{severity_color, severity_label};
    use deepseek_mobile_core::PcDiagnosticSeverity;

    #[test]
    fn diagnostic_severity_styles_are_stable() {
        assert_eq!(severity_label(&PcDiagnosticSeverity::Error), "ERROR");
        assert_eq!(severity_color(&PcDiagnosticSeverity::Warning), "#fcd34d");
    }
}
