use crate::mobile_drawer::CockpitSection;
use crate::pc_pairing_panel::pc_pairing_panel;
use crate::pc_pairing_state::PcPairingUiState;
use crate::project_files_panel::project_files_panel;
use crate::project_files_state::ProjectFilesUiState;
use dioxus::prelude::*;

pub fn cockpit_section_panel(
    section: CockpitSection,
    pc_pairing_state: &PcPairingUiState,
    project_files_state: Signal<ProjectFilesUiState>,
) -> Element {
    match section {
        CockpitSection::Chat => chat_empty_state(),
        CockpitSection::PcHost => pc_pairing_panel(pc_pairing_state),
        CockpitSection::Files => project_files_panel(project_files_state),
        CockpitSection::Terminal => placeholder_panel(
            "Terminal",
            "Command output from PC-host, Termux or remote executors.",
            &["Run task", "Stream output", "Stop command", "Copy logs"],
        ),
        CockpitSection::Approvals => placeholder_panel(
            "Approvals",
            "Risky actions from the AI agent will require explicit confirmation here.",
            &["Approve once", "Approve for session", "Deny", "Show diff"],
        ),
        CockpitSection::Git => placeholder_panel(
            "Git & GitHub",
            "Repository status, commits, branches, push/pull and pull requests.",
            &["Status", "Diff", "Commit", "Push / Pull"],
        ),
        CockpitSection::Settings => placeholder_panel(
            "Settings",
            "DeepSeek API key, GitHub, cloud disks, PC-host and security settings.",
            &["DeepSeek API", "GitHub", "Cloud disks", "Security"],
        ),
    }
}

fn chat_empty_state() -> Element {
    rsx! {
        div {
            color: "#9ca3af",
            text_align: "center",
            margin_top: "32px",
            white_space: "pre-wrap",
            "Ask DeepSeek to build, inspect, fix, test or deploy a project.\nOpen the drawer for PC Host, Files, Terminal, Git and Settings."
        }
    }
}

fn placeholder_panel(title: &'static str, description: &'static str, actions: &'static [&'static str]) -> Element {
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
                font_size: "20px",
                font_weight: "bold",
                "{title}"
            }
            div {
                color: "#9ca3af",
                font_size: "13px",
                white_space: "pre-wrap",
                "{description}"
            }
            div {
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                for action in actions {
                    div {
                        background_color: "#1f2937",
                        border: "1px solid #374151",
                        border_radius: "12px",
                        padding: "10px 12px",
                        font_size: "14px",
                        "{action}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mobile_drawer::CockpitSection;

    #[test]
    fn all_non_chat_sections_have_titles() {
        assert_eq!(CockpitSection::PcHost.title(), "PC Host");
        assert_eq!(CockpitSection::Files.title(), "Files");
        assert_eq!(CockpitSection::Terminal.title(), "Terminal");
        assert_eq!(CockpitSection::Approvals.title(), "Approvals");
        assert_eq!(CockpitSection::Git.title(), "Git & GitHub");
        assert_eq!(CockpitSection::Settings.title(), "Settings");
    }
}