//! Aggregated runtime health for the cockpit health panel.

use crate::mcp_state::McpUiState;
use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::NativeBridgeState;
use crate::pc_pairing_state::PcPairingUiState;
use crate::settings_state::SettingsFormState;
use crate::termux_state::TermuxWorkspaceState;
use deepseek_mobile_core::config::ExecutionMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeHealthSnapshot {
    pub api_configured: bool,
    pub execution_mode: ExecutionMode,
    pub pc_status_label: String,
    pub pc_online: bool,
    pub pc_workspace_active: bool,
    pub termux_configured: bool,
    pub termux_valid: bool,
    pub mcp_servers_connected: usize,
    pub mcp_servers_total: usize,
    pub native_pending: bool,
    pub native_last_error: Option<String>,
    pub recommendations: Vec<String>,
    pub network_hints: Vec<String>,
    /// API + valid Termux path — full agent on phone without PC.
    pub full_agent_on_phone_ready: bool,
    pub data_dir_display: String,
}

impl RuntimeHealthSnapshot {
    pub fn collect(
        settings: &SettingsFormState,
        pc: &PcPairingUiState,
        termux: &TermuxWorkspaceState,
        mcp: &McpUiState,
        bridge: &NativeBridgeState,
    ) -> Self {
        let api_configured =
            !settings.api_key.trim().is_empty() && settings.api_key.trim().starts_with("sk-");
        let pc_online = pc
            .active_endpoint
            .as_ref()
            .map(|e| e.is_healthy())
            .unwrap_or(false);
        let pc_workspace_active = pc.active_workspace_connection().is_some();
        let termux_configured = !termux.workspace_path.trim().is_empty();
        let termux_valid = termux.is_valid();

        let mcp_servers_total = mcp.registry.servers.len();
        let mcp_servers_connected = mcp
            .registry
            .servers
            .iter()
            .filter(|s| s.status == deepseek_mobile_core::McpServerStatus::Connected)
            .count();

        let pc_status_label = match &pc.status {
            crate::pc_pairing_state::PcPairingUiStatus::Online => "Online".to_string(),
            crate::pc_pairing_state::PcPairingUiStatus::Offline => "Offline".to_string(),
            crate::pc_pairing_state::PcPairingUiStatus::WaitingForPc => {
                "Waiting for PC host".to_string()
            }
            crate::pc_pairing_state::PcPairingUiStatus::Exported => {
                "Bundle exported — start PC host".to_string()
            }
            crate::pc_pairing_state::PcPairingUiStatus::ReadyToExport => {
                "Ready to export pairing".to_string()
            }
            crate::pc_pairing_state::PcPairingUiStatus::NotConfigured => {
                "Not configured".to_string()
            }
            crate::pc_pairing_state::PcPairingUiStatus::Error(message) => {
                format!("Error: {message}")
            }
        };

        let native_pending = bridge.has_pending_commands()
            || bridge.is_waiting_for_document_picker_callback()
            || bridge.is_waiting_for_termux_callback()
            || bridge.is_waiting_for_pc_discovery_callback();

        let full_agent_on_phone_ready = api_configured && termux_valid;

        let mut recommendations = Vec::new();
        if !api_configured {
            recommendations.push("Add a DeepSeek API key in Settings.".to_string());
        }
        if !termux_valid {
            recommendations.push(
                "Full agent on phone: install Termux, set allow-external-apps, then save a valid project path in Settings (e.g. /data/data/com.termux/files/home/project).".to_string(),
            );
        } else if !full_agent_on_phone_ready {
            recommendations.push(
                "Complete API key and Termux path to unlock the full on-device agent.".to_string(),
            );
        }
        if termux_valid && pc_workspace_active && !pc_online {
            recommendations.push(
                "Optional PC boost: saved PC workspace is offline — start deepseek-pc-host if you still need the desktop project.".to_string(),
            );
        } else if !termux_valid && !pc_workspace_active {
            recommendations.push(
                "Optional: PC Host panel — only for very large repos when Termux is not enough (not required for a full phone agent).".to_string(),
            );
        }
        if mcp_servers_total > 0 && mcp_servers_connected == 0 {
            recommendations.push(
                "MCP servers are configured but none are connected — open MCP panel and connect."
                    .to_string(),
            );
        }
        if settings.execution_mode == ExecutionMode::Plan {
            recommendations.push(
                "Plan mode is on — tools will not run until you switch to Agent mode.".to_string(),
            );
        }

        let network_hints = if pc_workspace_active {
            vec![
                "Optional PC Host URLs:".to_string(),
                "LAN: http://<pc-lan-ip>:8787 (same Wi‑Fi)".to_string(),
                "Tailscale: http://<machine>.ts.net:8787".to_string(),
            ]
        } else {
            Vec::new()
        };

        Self {
            api_configured,
            execution_mode: settings.execution_mode.clone(),
            pc_status_label,
            pc_online,
            pc_workspace_active,
            termux_configured,
            termux_valid,
            mcp_servers_connected,
            mcp_servers_total,
            native_pending,
            native_last_error: bridge.last_error.clone(),
            recommendations,
            network_hints,
            full_agent_on_phone_ready,
            data_dir_display: default_data_dir().display().to_string(),
        }
    }

    pub fn overall_ready(&self) -> bool {
        self.api_configured
    }

    /// Full TUI-like agent on the phone (Termux), without PC.
    pub fn full_agent_on_phone_ready(&self) -> bool {
        self.full_agent_on_phone_ready
    }

    /// Optional PC workstation boost is connected.
    pub fn pc_boost_ready(&self) -> bool {
        self.api_configured && self.pc_workspace_active && self.pc_online
    }

    pub fn workstation_ready(&self) -> bool {
        self.pc_boost_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeHealthSnapshot;
    use crate::mcp_state::McpUiState;
    use crate::native_bridge::NativeBridgeState;
    use crate::pc_pairing_state::PcPairingUiState;
    use crate::settings_state::SettingsFormState;
    use crate::termux_state::TermuxWorkspaceState;

    #[test]
    fn recommends_api_key_when_missing() {
        let mut settings = SettingsFormState::default();
        settings.api_key.clear();
        let snapshot = RuntimeHealthSnapshot::collect(
            &settings,
            &PcPairingUiState::default(),
            &TermuxWorkspaceState::default(),
            &McpUiState::default(),
            &NativeBridgeState::default(),
        );
        assert!(!snapshot.api_configured);
        assert!(snapshot
            .recommendations
            .iter()
            .any(|line| line.contains("API key")));
    }

    #[test]
    fn recommends_termux_not_pc_as_primary_path() {
        let snapshot = RuntimeHealthSnapshot::collect(
            &SettingsFormState::default(),
            &PcPairingUiState::default(),
            &TermuxWorkspaceState::default(),
            &McpUiState::default(),
            &NativeBridgeState::default(),
        );
        assert!(!snapshot.full_agent_on_phone_ready);
        assert!(snapshot
            .recommendations
            .iter()
            .any(|line| line.contains("Termux")));
        assert!(snapshot
            .recommendations
            .iter()
            .any(|line| line.contains("Optional") && line.contains("PC Host")));
    }
}
