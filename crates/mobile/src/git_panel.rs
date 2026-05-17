use crate::git_state::GitUiState;
use dioxus::prelude::*;

pub fn git_panel(state: &GitUiState) -> Element {
    let dirty = state.is_dirty();

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

            // Header
            div {
                background_color: "#0f172a",
                border: "1px solid #334155",
                border_radius: "14px",
                padding: "12px",
                div {
                    display: "flex", justify_content: "space_between", align_items: "center",
                    div { font_size: "18px", font_weight: "bold", "Git & GitHub" }
                    div {
                        color: if dirty { "#fbbf24" } else { "#6b7280" },
                        font_size: "12px",
                        "branch: {state.current_branch}"
                    }
                }
            }

            // Status
            SectionCard {
                title: "Status",
                count: Some(state.changed_files),
                action_label: "Refresh",
            }
            if !state.status_text.is_empty() {
                div {
                    background_color: "#0d1117",
                    border: "1px solid #30363d",
                    border_radius: "8px",
                    padding: "10px",
                    font_family: "monospace",
                    font_size: "12px",
                    white_space: "pre_wrap",
                    max_height: "180px",
                    overflow_y: "auto",
                    "{state.status_text}"
                }
            }

            // Diff
            SectionCard {
                title: "Diff",
                count: Some(state.diff_text.lines().count()),
                action_label: "Refresh",
            }
            if !state.diff_text.is_empty() {
                DiffBlock { text: state.diff_text.clone() }
            }

            // Branches
            SectionCard {
                title: "Branches",
                count: Some(state.branch_list.lines().count()),
                action_label: "List",
            }
            if !state.branch_list.is_empty() {
                div {
                    background_color: "#0d1117",
                    border: "1px solid #30363d",
                    border_radius: "8px",
                    padding: "10px",
                    font_family: "monospace",
                    font_size: "12px",
                    white_space: "pre_wrap",
                    max_height: "120px",
                    overflow_y: "auto",
                    "{state.branch_list}"
                }
            }

            // Commit
            SectionCard {
                title: "Commit",
                subtitle: if dirty { "Ready to commit" } else { "No changes" },
                action_label: if dirty { "Commit" } else { "---" },
            }

            // Push / Pull
            div {
                display: "flex", gap: "12px",
                SectionCard { title: "Push", subtitle: "origin", action_label: "Push" }
                SectionCard { title: "Pull", subtitle: "origin", action_label: "Pull" }
            }

            // Error
            if let Some(error) = state.error.as_ref() {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "12px",
                    padding: "12px",
                    color: "#fca5a5",
                    font_size: "13px",
                    "{error}"
                }
            }

            // Loading
            if state.loading {
                div {
                    color: "#6b7280", font_size: "13px", text_align: "center",
                    "Running git operation..."
                }
            }
        }
    }
}

#[component]
fn SectionCard(title: String, subtitle: Option<String>, action_label: String, count: Option<usize>) -> Element {
    rsx! {
        div {
            background_color: "#0f172a",
            border: "1px solid #334155",
            border_radius: "14px",
            padding: "12px",
            display: "flex",
            justify_content: "space_between",
            align_items: "center",

            div {
                display: "flex", flex_direction: "column", gap: "4px",
                div {
                    font_size: "14px", font_weight: "600",
                    "{title}"
                    if let Some(c) = count {
                        if c > 0 {
                            span { color: "#fbbf24", font_size: "12px", margin_left: "8px", "({c})" }
                        }
                    }
                }
                if let Some(s) = subtitle {
                    if !s.is_empty() {
                        div { color: "#9ca3af", font_size: "12px", "{s}" }
                    }
                }
            }

            button {
                background_color: if action_label == "---" { "#1f2937" } else { "#2563eb" },
                border: "none",
                border_radius: "10px",
                padding: "6px 14px",
                color: if action_label == "---" { "#6b7280" } else { "white" },
                font_size: "13px",
                font_weight: "500",
                "{action_label}"
            }
        }
    }
}

#[component]
fn DiffBlock(text: String) -> Element {
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    rsx! {
        div {
            background_color: "#0d1117",
            border: "1px solid #30363d",
            border_radius: "8px",
            padding: "10px",
            font_family: "monospace",
            font_size: "12px",
            line_height: "1.6",
            max_height: "200px",
            overflow_y: "auto",
            white_space: "pre_wrap",
            for line in lines {
                div {
                    color: if line.starts_with('+') { "#3fb950" }
                    else if line.starts_with('-') { "#f85149" }
                    else if line.starts_with("@@") { "#58a6ff" }
                    else { "#c9d1d9" },
                    "{line}"
                }
            }
        }
    }
}
