use crate::diagnostics_panel::diagnostics_panel;
use crate::diagnostics_state::DiagnosticsUiState;
use crate::mobile_approval_panel::mobile_approval_panel;
use crate::mobile_drawer::CockpitSection;
use crate::native_bridge::NativeBridgeState;
use crate::pc_pairing_panel::pc_pairing_panel;
use crate::pc_pairing_state::PcPairingUiState;
use crate::project_files_panel::project_files_panel;
use crate::project_files_state::ProjectFilesUiState;
use crate::git_panel::git_panel;
use crate::git_state::GitUiState;
use crate::settings_panel::settings_panel;
use crate::settings_state::SettingsFormState;
use crate::snapshots_panel::snapshots_panel;
use crate::snapshots_state::SnapshotsUiState;
use crate::terminal_panel::terminal_panel;
use crate::terminal_state::TerminalUiState;
use deepseek_mobile_core::{ApprovalCardView, ReviewDecision};
use dioxus::prelude::*;

pub fn cockpit_section_panel(
    section: CockpitSection,
    approval_cards: Signal<Vec<ApprovalCardView>>,
    pc_pairing_state: Signal<PcPairingUiState>,
    mut native_bridge: Signal<NativeBridgeState>,
    project_files_state: Signal<ProjectFilesUiState>,
    snapshots_state: Signal<SnapshotsUiState>,
    diagnostics_state: Signal<DiagnosticsUiState>,
    git_state: Signal<GitUiState>,
    mut terminal_state: Signal<TerminalUiState>,
    settings_state: Signal<SettingsFormState>,
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
) -> Element {
    match section {
        CockpitSection::Chat => chat_empty_state(),
        CockpitSection::PcHost => pc_pairing_panel(pc_pairing_state, native_bridge),
        CockpitSection::Files => project_files_panel(project_files_state),
        CockpitSection::Snapshots => snapshots_panel(snapshots_state),
        CockpitSection::Diagnostics => diagnostics_panel(&diagnostics_state()),
        CockpitSection::Terminal => terminal_panel(
            terminal_state,
            EventHandler::new(move |_| {
                let mut bridge = native_bridge.write();
                bridge.enqueue_open_terminal("default");
            }),
            EventHandler::new(move |input: String| {
                let ts = terminal_state.write();
                if let Some(session_id) = ts.selected_session_id.clone() {
                    let mut bridge = native_bridge.write();
                    bridge.enqueue_terminal_input(session_id, input);
                }
            }),
            EventHandler::new(move |session_id: String| {
                let mut bridge = native_bridge.write();
                bridge.enqueue_close_terminal(session_id);
            }),
        ),
        CockpitSection::Approvals => {
            let cards = approval_cards();
            if cards.is_empty() {
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
                        div { font_size: "20px", font_weight: "bold", "Approvals" }
                        div { color: "#9ca3af", font_size: "13px", "No pending approvals." }
                    }
                }
            } else {
                rsx! {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "10px",
                        padding: "8px 0",
                        div {
                            font_size: "20px",
                            font_weight: "bold",
                            color: "white",
                            "Approvals ({cards.len()})"
                        }
                        {mobile_approval_panel(
                            &cards,
                            EventHandler::new(move |(id, decision)| {
                                on_approval_decision.call((id, decision));
                            }),
                        )}
                    }
                }
            }
        },
        CockpitSection::Git => git_panel(&git_state()),
        CockpitSection::Settings => settings_panel(settings_state),
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

#[cfg(test)]
mod tests {
    use crate::mobile_drawer::CockpitSection;

    #[test]
    fn all_non_chat_sections_have_titles() {
        assert_eq!(CockpitSection::PcHost.title(), "PC Host");
        assert_eq!(CockpitSection::Files.title(), "Files");
        assert_eq!(CockpitSection::Snapshots.title(), "Snapshots");
        assert_eq!(CockpitSection::Diagnostics.title(), "Diagnostics");
        assert_eq!(CockpitSection::Terminal.title(), "Terminal");
        assert_eq!(CockpitSection::Approvals.title(), "Approvals");
        assert_eq!(CockpitSection::Git.title(), "Git & GitHub");
        assert_eq!(CockpitSection::Settings.title(), "Settings");
    }
}
