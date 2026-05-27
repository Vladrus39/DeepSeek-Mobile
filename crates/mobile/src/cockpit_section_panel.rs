use crate::diagnostics_panel::diagnostics_panel;
use crate::diagnostics_state::DiagnosticsUiState;
use crate::document_picker::DocumentPickerState;
use crate::git_panel::git_panel;
use crate::git_state::{GitPanelAction, GitUiState};
use crate::health_panel::{health_panel, HealthQuickAction};
use crate::locale::{pick, AppLanguage};
use crate::mcp_panel::mcp_panel;
use crate::mcp_state::McpUiState;
use crate::mobile_approval_panel::mobile_approval_panel;
use crate::mobile_drawer::CockpitSection;
use crate::mobile_git_runner::{apply_git_action_result, run_mobile_git_action};
use crate::mobile_runtime_config::MobileRuntimeConfig;
use crate::native_bridge::NativeBridgeState;
use crate::pc_pairing_panel::pc_pairing_panel;
use crate::pc_pairing_state::PcPairingUiState;
use crate::project_files_panel::{project_files_panel, PcFileBrowserConnection};
use crate::project_files_state::ProjectFilesUiState;
use crate::project_transfer_state::ProjectTransferState;
use crate::runtime_health::RuntimeHealthSnapshot;
use crate::settings_panel::settings_panel;
use crate::settings_state::SettingsFormState;
use crate::skills_panel::skills_panel;
use crate::skills_state::SkillsUiState;
use crate::snapshots_panel::snapshots_panel;
use crate::snapshots_state::SnapshotsUiState;
use crate::tasks_panel::tasks_panel;
use crate::tasks_state::TasksUiState;
use crate::terminal_panel::terminal_panel;
use crate::terminal_state::TerminalUiState;
use crate::termux_state::TermuxWorkspaceState;
use deepseek_mobile_core::{ApprovalCardView, ReviewDecision};
use dioxus::prelude::*;

pub fn cockpit_section_panel(
    section: CockpitSection,
    approval_cards: Signal<Vec<ApprovalCardView>>,
    pc_pairing_state: Signal<PcPairingUiState>,
    mut native_bridge: Signal<NativeBridgeState>,
    picker_state: Signal<DocumentPickerState>,
    project_files_state: Signal<ProjectFilesUiState>,
    project_transfer_state: Signal<ProjectTransferState>,
    snapshots_state: Signal<SnapshotsUiState>,
    diagnostics_state: Signal<DiagnosticsUiState>,
    mut git_state: Signal<GitUiState>,
    mut terminal_state: Signal<TerminalUiState>,
    mcp_state: Signal<McpUiState>,
    skills_state: Signal<SkillsUiState>,
    tasks_state: Signal<TasksUiState>,
    settings_state: Signal<SettingsFormState>,
    termux_state: Signal<TermuxWorkspaceState>,
    lang: Signal<AppLanguage>,
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
    on_health_quick_action: EventHandler<HealthQuickAction>,
) -> Element {
    match section {
        CockpitSection::Chat => chat_empty_state(lang()),
        CockpitSection::PcHost => pc_pairing_panel(pc_pairing_state, native_bridge),
        CockpitSection::Files => {
            let pc_connection =
                pc_pairing_state()
                    .active_workspace_connection()
                    .and_then(|connection| {
                        connection
                            .pc_gateway
                            .clone()
                            .map(|config| PcFileBrowserConnection {
                                client: deepseek_mobile_core::PcGatewayClient::new(config),
                                workspace_id: connection.workspace_id.clone(),
                                workspace_root: connection.workspace_root.display().to_string(),
                            })
                    });
            project_files_panel(
                project_files_state,
                approval_cards(),
                pc_connection,
                picker_state,
                native_bridge,
                project_transfer_state,
            )
        }
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
                let l = lang();
                let title = pick(l, "Одобрения", "Approvals");
                let body = pick(l, "Нет ожидающих одобрений.", "No pending approvals.");
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
                        div { font_size: "20px", font_weight: "bold", "{title}" }
                        div { color: "#9ca3af", font_size: "13px", "{body}" }
                    }
                }
            } else {
                let approvals_title = if lang() == AppLanguage::Ru {
                    format!("Одобрения ({})", cards.len())
                } else {
                    format!("Approvals ({})", cards.len())
                };
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
                            "{approvals_title}"
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
        }
        CockpitSection::Git => git_panel(
            &git_state(),
            EventHandler::new(move |action: GitPanelAction| {
                let runtime = MobileRuntimeConfig::default();
                let current_state = git_state();
                git_state.write().set_loading();
                spawn(async move {
                    match run_mobile_git_action(action, runtime, current_state).await {
                        Ok(result) => {
                            let mut state = git_state();
                            apply_git_action_result(&mut state, result);
                            git_state.set(state);
                        }
                        Err(error) => {
                            git_state.write().set_error(error.to_string());
                        }
                    }
                });
            }),
            EventHandler::new(move |message: String| {
                git_state.write().set_commit_message(message);
            }),
        ),
        CockpitSection::Mcp => mcp_panel(lang(), mcp_state),
        CockpitSection::Skills => skills_panel(lang(), skills_state),
        CockpitSection::Tasks => {
            let pc_client = pc_pairing_state()
                .active_workspace_connection()
                .and_then(|connection| connection.pc_gateway.clone())
                .map(deepseek_mobile_core::PcGatewayClient::new);
            tasks_panel(tasks_state, pc_client)
        }
        CockpitSection::Health => health_panel(
            lang(),
            RuntimeHealthSnapshot::collect(
                &settings_state(),
                &pc_pairing_state(),
                &termux_state(),
                &mcp_state(),
                &native_bridge(),
            ),
            on_health_quick_action,
        ),
        CockpitSection::Settings => settings_panel(lang, settings_state, termux_state),
    }
}

fn chat_empty_state(lang: AppLanguage) -> Element {
    let hint = pick(
        lang,
        "Попросите DeepSeek собрать, проверить, исправить или развернуть проект.\nОткройте меню: PC Host, Файлы, Терминал, Git, Настройки.",
        "Ask DeepSeek to build, inspect, fix, test or deploy a project.\nOpen the drawer for PC Host, Files, Terminal, Git and Settings.",
    );
    rsx! {
        div {
            color: "#9ca3af",
            text_align: "center",
            margin_top: "32px",
            white_space: "pre-wrap",
            "{hint}"
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
        assert_eq!(CockpitSection::Mcp.title(), "MCP");
        assert_eq!(CockpitSection::Skills.title(), "Skills");
        assert_eq!(CockpitSection::Tasks.title(), "Tasks");
        assert_eq!(CockpitSection::Settings.title(), "Settings");
    }
}
