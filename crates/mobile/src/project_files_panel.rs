use crate::project_diff::{build_text_diff_preview, diff_line_color};
use crate::project_files::{ProjectEntryKind, ProjectFilePreview, ProjectTreeSnapshot};
use crate::project_files_state::ProjectFilesUiState;
use deepseek_mobile_core::ApprovalCardView;
use dioxus::prelude::*;

pub fn project_files_panel(mut state: Signal<ProjectFilesUiState>, approval_cards: Vec<ApprovalCardView>) -> Element {
    if !state().loaded {
        let mut next = state();
        next.refresh();
        next.set_pending_diffs(&approval_cards);
        state.set(next);
    }

    let snapshot = state().snapshot.clone();
    let preview = state().preview.clone();
    let selected_path = state().selected_path.clone();
    let last_error = state().last_error.clone();

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

            {header_card(&snapshot, state)}
            if let Some(error) = last_error {
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
            {tree_card(&snapshot, selected_path.as_deref(), state)}
            {file_preview_card(preview.as_ref())}
            {diff_preview_card(&state(), &approval_cards)}
        }
    }
}

fn header_card(snapshot: &ProjectTreeSnapshot, mut state: Signal<ProjectFilesUiState>) -> Element {
    rsx! {
        div {
            background_color: "#0f172a",
            border: "1px solid #334155",
            border_radius: "14px",
            padding: "12px",
            display: "flex",
            flex_direction: "column",
            gap: "8px",

            div {
                display: "flex",
                justify_content: "space-between",
                align_items: "center",
                gap: "8px",
                div { font_size: "18px", font_weight: "bold", "Project files" }
                button {
                    background_color: "#1d4ed8",
                    color: "white",
                    border: "1px solid #3b82f6",
                    border_radius: "999px",
                    padding: "6px 10px",
                    font_size: "12px",
                    onclick: move |_| {
                        let mut next = state();
                        next.refresh();
                        state.set(next);
                    },
                    "Refresh"
                }
            }
            div { color: "#9ca3af", font_size: "12px", "Root: {snapshot.root}" }
            div {
                display: "flex",
                gap: "8px",
                flex_wrap: "wrap",
                {stat_badge("Dirs", snapshot.directory_count().to_string())}
                {stat_badge("Files", snapshot.file_count().to_string())}
                if snapshot.truncated {
                    {stat_badge("Tree", "truncated".to_string())}
                } else {
                    {stat_badge("Tree", "complete".to_string())}
                }
            }
        }
    }
}

fn stat_badge(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            background_color: "#1f2937",
            border: "1px solid #374151",
            border_radius: "999px",
            padding: "5px 9px",
            font_size: "12px",
            "{label}: {value}"
        }
    }
}

fn tree_card(
    snapshot: &ProjectTreeSnapshot,
    selected_path: Option<&str>,
    mut state: Signal<ProjectFilesUiState>,
) -> Element {
    let browsing_dir = state().browsing_dir.clone();
    rsx! {
        div {
            background_color: "#020617",
            border: "1px solid #1f2937",
            border_radius: "14px",
            padding: "10px",
            display: "flex",
            flex_direction: "column",
            gap: "6px",

            div {
                display: "flex",
                justify_content: "space-between",
                align_items: "center",
                gap: "8px",
                div { font_size: "14px", font_weight: "bold", "Workspace tree" }
                if !browsing_dir.is_empty() {
                    button {
                        background_color: "#1f2937",
                        color: "white",
                        border: "1px solid #374151",
                        border_radius: "999px",
                        padding: "4px 8px",
                        font_size: "11px",
                        onclick: move |_| {
                            let mut next = state();
                            next.navigate_up();
                            state.set(next);
                        },
                        "\u{2190} Up"
                    }
                }
            }

            div { color: "#9ca3af", font_size: "11px", "{state().current_browsing_display()}" }

            if snapshot.entries.is_empty() {
                div { color: "#9ca3af", font_size: "12px", "No files found in this directory." }
            } else {
                for entry in snapshot.entries.iter().take(120) {
                    if matches!(entry.kind, ProjectEntryKind::File) {
                        button {
                            background_color: if selected_path == Some(entry.path.as_str()) { "#1e3a8a" } else { "#111827" },
                            color: "white",
                            border: if selected_path == Some(entry.path.as_str()) { "1px solid #3b82f6" } else { "1px solid #1f2937" },
                            border_radius: "10px",
                            padding: "6px 8px",
                            display: "flex",
                            justify_content: "space-between",
                            gap: "8px",
                            text_align: "left",
                            onclick: {
                                let path = entry.path.clone();
                                move |_| {
                                    let mut next = state();
                                    next.open_file(path.clone());
                                    state.set(next);
                                }
                            },

                            div {
                                color: "#e5e7eb",
                                font_size: "12px",
                                white_space: "nowrap",
                                overflow: "hidden",
                                text_overflow: "ellipsis",
                                "\u{2022} {entry.name}"
                            }
                            if let Some(size_bytes) = entry.size_bytes {
                                div { color: "#6b7280", font_size: "11px", "{size_bytes} B" }
                            }
                        }
                    } else {
                        button {
                            display: "flex",
                            justify_content: "space-between",
                            gap: "8px",
                            padding: "6px 8px",
                            border_radius: "10px",
                            border: "1px solid transparent",
                            background_color: "#0f172a",
                            text_align: "left",
                            color: "white",
                            onclick: {
                                let path = entry.path.clone();
                                let browsing = state().browsing_dir.clone();
                                let target = if browsing.is_empty() {
                                    path.clone()
                                } else {
                                    format!("{}/{}", browsing, path)
                                };
                                move |_| {
                                    let mut next = state();
                                    next.navigate_to_dir(target.clone());
                                    state.set(next);
                                }
                            },

                            div {
                                color: "#93c5fd",
                                font_size: "12px",
                                white_space: "nowrap",
                                overflow: "hidden",
                                text_overflow: "ellipsis",
                                "\u{25b8} {entry.name}/"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn file_preview_card(preview: Option<&ProjectFilePreview>) -> Element {
    rsx! {
        div {
            background_color: "#020617",
            border: "1px solid #1f2937",
            border_radius: "14px",
            padding: "10px",
            display: "flex",
            flex_direction: "column",
            gap: "8px",

            div { font_size: "14px", font_weight: "bold", "Open file" }
            if let Some(preview) = preview {
                div { color: "#93c5fd", font_size: "12px", "{preview.path} · {preview.line_count} lines · {preview.size_bytes} B" }
                pre {
                    background_color: "#0b1120",
                    border: "1px solid #1e293b",
                    border_radius: "12px",
                    padding: "10px",
                    overflow_x: "auto",
                    max_height: "260px",
                    white_space: "pre-wrap",
                    font_size: "12px",
                    color: "#d1d5db",
                    "{preview.content}"
                }
            } else {
                div { color: "#9ca3af", font_size: "12px", "Tap a source/text file to preview it here." }
            }
        }
    }
}

fn diff_preview_card(state: &ProjectFilesUiState, approval_cards: &[ApprovalCardView]) -> Element {
    // Compute diff reactively from approval cards for the selected file
    let diff = state.selected_path.as_ref().and_then(|selected| {
        for card in approval_cards {
            let card_path = first_string_arg(card, &["path", "file", "file_path", "relative_path", "target_path"]);
            if card_path.as_deref() != Some(selected.as_str()) {
                continue;
            }
            if let Some(after) = first_string_arg(card, &["content", "new_content", "after", "replacement", "text"]) {
                let before = first_string_arg(card, &["before", "old_content", "current_content"]).unwrap_or_default();
                return Some(build_text_diff_preview(selected.clone(), &before, &after));
            }
            if let Some(search) = first_string_arg(card, &["search", "old_text"]) {
                let replace = first_string_arg(card, &["replace", "new_text"]).unwrap_or_default();
                let current = state.preview.as_ref().map(|p| p.content.as_str()).unwrap_or("");
                let after = current.replacen(&search, &replace, 1);
                return Some(build_text_diff_preview(selected.clone(), current, &after));
            }
        }
        None
    });

    rsx! {
        div {
            background_color: "#020617",
            border: "1px solid #1f2937",
            border_radius: "14px",
            padding: "10px",
            display: "flex",
            flex_direction: "column",
            gap: "8px",

            div { font_size: "14px", font_weight: "bold", "Diff preview" }
            if let Some(diff) = diff {
                div { color: "#9ca3af", font_size: "12px",
                    "{diff.path} · +{diff.added_lines} / -{diff.removed_lines}"
                }
                div {
                    background_color: "#0b1120",
                    border: "1px solid #1e293b",
                    border_radius: "12px",
                    padding: "10px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "2px",

                    for line in diff.lines.iter().take(80) {
                        div {
                            color: "{diff_line_color(&line.kind)}",
                            font_family: "monospace",
                            font_size: "12px",
                            white_space: "pre-wrap",
                            "{line.text}"
                        }
                    }
                }
            } else {
                if state.preview.is_some() {
                    div { color: "#9ca3af", font_size: "12px", "No pending changes for this file." }
                } else {
                    div { color: "#9ca3af", font_size: "12px", "Select a file to see pending change diffs." }
                }
            }
        }
    }
}

/// Extract the first matching string argument from an approval card's argument_preview.
fn first_string_arg(card: &ApprovalCardView, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = card.argument_preview.get(*key).and_then(|v| v.as_str()) {
            if !value.trim().is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::project_diff::build_text_diff_preview;

    #[test]
    fn panel_diff_preview_model_is_non_empty_for_changed_text() {
        let diff = build_text_diff_preview("README.md", "hello\n", "hello\nworld\n");
        assert!(!diff.is_empty());
        assert_eq!(diff.added_lines, 1);
    }
}