use crate::project_diff::{build_text_diff_preview, diff_line_color};
use crate::project_files::{
    choose_default_preview_file, read_project_file, scan_project_tree, ProjectEntryKind,
    ProjectFilePreview, ProjectTreeSnapshot, DEFAULT_MAX_FILE_BYTES, DEFAULT_MAX_TREE_ENTRIES,
};
use dioxus::prelude::*;
use std::path::PathBuf;

const DEFAULT_WORKSPACE_ROOT: &str = ".";

pub fn project_files_panel() -> Element {
    let workspace_root = PathBuf::from(DEFAULT_WORKSPACE_ROOT);
    let snapshot = scan_project_tree(&workspace_root, DEFAULT_MAX_TREE_ENTRIES).unwrap_or(ProjectTreeSnapshot {
        root: workspace_root.to_string_lossy().to_string(),
        entries: Vec::new(),
        truncated: false,
    });
    let selected_path = choose_default_preview_file(&snapshot);
    let preview = selected_path
        .as_deref()
        .and_then(|path| read_project_file(&workspace_root, path, DEFAULT_MAX_FILE_BYTES).ok());

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

            {header_card(&snapshot)}
            {tree_card(&snapshot)}
            {file_preview_card(preview.as_ref())}
            {diff_preview_card(preview.as_ref())}
        }
    }
}

fn header_card(snapshot: &ProjectTreeSnapshot) -> Element {
    rsx! {
        div {
            background_color: "#0f172a",
            border: "1px solid #334155",
            border_radius: "14px",
            padding: "12px",
            display: "flex",
            flex_direction: "column",
            gap: "6px",

            div { font_size: "18px", font_weight: "bold", "Project files" }
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

fn tree_card(snapshot: &ProjectTreeSnapshot) -> Element {
    rsx! {
        div {
            background_color: "#020617",
            border: "1px solid #1f2937",
            border_radius: "14px",
            padding: "10px",
            display: "flex",
            flex_direction: "column",
            gap: "6px",

            div { font_size: "14px", font_weight: "bold", "Workspace tree" }

            if snapshot.entries.is_empty() {
                div { color: "#9ca3af", font_size: "12px", "No files found in workspace root yet." }
            } else {
                for entry in snapshot.entries.iter().take(80) {
                    div {
                        display: "flex",
                        justify_content: "space-between",
                        gap: "8px",
                        padding: "6px 8px",
                        border_radius: "10px",
                        background_color: if matches!(entry.kind, ProjectEntryKind::Directory) { "#0f172a" } else { "#111827" },
                        margin_left: "{entry.depth * 12}px",

                        div {
                            color: if matches!(entry.kind, ProjectEntryKind::Directory) { "#93c5fd" } else { "#e5e7eb" },
                            font_size: "12px",
                            white_space: "nowrap",
                            overflow: "hidden",
                            text_overflow: "ellipsis",
                            if matches!(entry.kind, ProjectEntryKind::Directory) {
                                "▸ {entry.name}"
                            } else {
                                "• {entry.name}"
                            }
                        }
                        if let Some(size_bytes) = entry.size_bytes {
                            div { color: "#6b7280", font_size: "11px", "{size_bytes} B" }
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
                div { color: "#9ca3af", font_size: "12px", "Select or create a text/source file to preview it here." }
            }
        }
    }
}

fn diff_preview_card(preview: Option<&ProjectFilePreview>) -> Element {
    let diff = preview.map(|preview| {
        let after = format!("{}\n// Proposed change preview hook\n", preview.content.trim_end());
        build_text_diff_preview(preview.path.clone(), &preview.content, &after)
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
                div { color: "#9ca3af", font_size: "12px", "{diff.path} · +{diff.added_lines} / -{diff.removed_lines}" }
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
                div { color: "#9ca3af", font_size: "12px", "AI patch diffs will be shown here before approval." }
            }
        }
    }
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