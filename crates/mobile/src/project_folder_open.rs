//! Open the active project workspace folder in the OS file manager, with in-app fallback.

use crate::mobile_runtime_config::MobileRuntimeConfig;
use crate::pc_pairing_state::PcPairingUiState;
use crate::project_files_state::ProjectFilesUiState;
use deepseek_mobile_core::{PcGatewayClient, PcGatewayResponse};
use dioxus::prelude::*;

use crate::mobile_drawer::CockpitSection;

/// Resolve the on-disk folder path for the active workspace (phone sandbox, Termux, or PC root).
pub fn active_workspace_folder_path(pc_pairing: &PcPairingUiState) -> String {
    if let Some(connection) = pc_pairing.active_workspace_connection() {
        return connection.workspace_root.display().to_string();
    }
    MobileRuntimeConfig::default_mobile().workspace_root_display()
}

/// Try to open the workspace folder on a paired PC via gateway `open_path`.
pub async fn try_open_pc_workspace_folder(client: &PcGatewayClient, workspace_id: &str) -> bool {
    match client.open_path(workspace_id, ".").await {
        Ok(PcGatewayResponse::PathOpened { .. }) => true,
        _ => false,
    }
}

/// Switch to in-app Files with the project tree visible (import/export card collapsed).
pub fn show_in_app_files_focus_tree(
    mut active_section: Signal<CockpitSection>,
    mut chat_history_open: Signal<bool>,
    mut drawer_open: Signal<bool>,
    mut project_files_state: Signal<ProjectFilesUiState>,
    pc_pairing: &PcPairingUiState,
) {
    active_section.set(CockpitSection::Files);
    chat_history_open.set(false);
    drawer_open.set(false);

    if let Some(connection) = pc_pairing.active_workspace_connection() {
        if let Some(gateway_config) = connection.pc_gateway.clone() {
            let client = PcGatewayClient::new(gateway_config);
            let workspace_id = connection.workspace_id.clone();
            let workspace_root = connection.workspace_root.display().to_string();
            let mut files_signal = project_files_state;
            spawn(async move {
                let mut files = files_signal();
                files.focus_tree = true;
                files.workspace_root = workspace_root;
                files.reset_to_workspace_root();
                files.loaded = true;
                let _ = files.refresh_via_pc(&client, &workspace_id).await;
                files_signal.set(files);
            });
            return;
        }
    }

    let runtime = MobileRuntimeConfig::default_mobile();
    let mut files = project_files_state.write();
    files.focus_tree = true;
    files.workspace_root = runtime.workspace_root_display();
    files.reset_to_workspace_root();
    files.refresh();
}

pub fn clear_files_focus_tree_if_leaving(
    from: CockpitSection,
    to: CockpitSection,
    mut project_files_state: Signal<ProjectFilesUiState>,
) {
    if from == CockpitSection::Files && to != CockpitSection::Files {
        project_files_state.write().focus_tree = false;
    }
}
