use crate::pc_pairing_manager::{MobilePcPairingExport, MobilePcPairingRequest, PcPairingManager};
use deepseek_mobile_core::{
    PcGatewayConfig, PcGatewayDiscoveryCandidate, PcGatewayDiscoveryReport,
    PcGatewayDiscoveryService, PcGatewayDiscoveryStatus, PcGatewayEndpointCandidate,
    PcGatewayEndpointHealth, PcGatewayTransportMode, PcGatewayTrustLevel, WorkspaceConnection,
    WorkspaceConnectionStatus,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcPairingUiStatus {
    NotConfigured,
    ReadyToExport,
    Exported,
    WaitingForPc,
    Online,
    Offline,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcReconnectAction {
    ScanAgain,
    RetryActiveRoute,
    UseBestDiscoveredRoute,
    ForgetBadRoutes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcReconnectControl {
    pub action: PcReconnectAction,
    pub label: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcReconnectEffect {
    None,
    StartDiscovery { request_id: String },
    RetryRoute { base_url: String },
    SelectedRoute { base_url: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcPairingUiState {
    pub status: PcPairingUiStatus,
    pub request: Option<MobilePcPairingRequest>,
    pub export: Option<MobilePcPairingExport>,
    pub discovery_report: Option<PcGatewayDiscoveryReport>,
    pub active_endpoint: Option<PcGatewayEndpointHealth>,
    pub endpoint_health: Vec<PcGatewayEndpointHealth>,
    pub reconnect_generation: u64,
    pub last_reconnect_action: Option<PcReconnectAction>,
    pub last_error: Option<String>,
}

impl Default for PcPairingUiState {
    fn default() -> Self {
        Self {
            status: PcPairingUiStatus::NotConfigured,
            request: None,
            export: None,
            discovery_report: None,
            active_endpoint: None,
            endpoint_health: Vec::new(),
            reconnect_generation: 0,
            last_reconnect_action: None,
            last_error: None,
        }
    }
}

impl PcPairingUiState {
    pub fn configure(&mut self, request: MobilePcPairingRequest) {
        self.request = Some(request);
        self.export = None;
        self.discovery_report = None;
        self.active_endpoint = None;
        self.endpoint_health.clear();
        self.reconnect_generation = 0;
        self.last_reconnect_action = None;
        self.last_error = None;
        self.status = PcPairingUiStatus::ReadyToExport;
    }

    pub fn export_zip(&mut self, output_dir: impl AsRef<Path>) -> Option<PathBuf> {
        let Some(mut request) = self.request.clone() else {
            self.set_error("PC pairing request is not configured");
            return None;
        };
        if let Some(config) = crate::settings_state::load_saved_config() {
            if !config.trusted_external_paths.is_empty() {
                request.trusted_paths = config.trusted_external_paths;
            }
        }

        match PcPairingManager::export_zip(request, output_dir) {
            Ok(export) => {
                let zip_path = export.zip_path.clone();
                self.export = Some(export);
                self.last_error = None;
                self.status = PcPairingUiStatus::Exported;
                Some(zip_path)
            }
            Err(error) => {
                self.set_error(error.to_string());
                None
            }
        }
    }

    pub fn mark_waiting_for_pc(&mut self) {
        self.status = PcPairingUiStatus::WaitingForPc;
        self.last_error = None;
    }

    pub fn mark_online(&mut self) {
        self.status = PcPairingUiStatus::Online;
        self.last_error = None;
    }

    pub fn mark_offline(&mut self) {
        self.status = PcPairingUiStatus::Offline;
    }

    /// Probe a user-supplied base URL (LAN `http://IP:8787` or remote `https://host`).
    pub async fn probe_and_connect_manual_url(&mut self, raw_url: &str) -> Result<(), String> {
        let trimmed = raw_url.trim();
        if trimmed.is_empty() {
            return Err("Enter PC Host URL (e.g. http://192.168.1.10:8787 or https://your-tunnel)".to_string());
        }
        let service = PcGatewayDiscoveryService::new(true);
        let report = service.from_manual_base_url(trimmed, "manual");
        let candidate = report
            .candidates
            .first()
            .ok_or_else(|| "No candidate built from URL".to_string())?;
        if candidate.status == PcGatewayDiscoveryStatus::Rejected {
            return Err(candidate
                .message
                .clone()
                .unwrap_or_else(|| "URL rejected by transport policy".to_string()));
        }
        let probed = service.probe_candidates(report).await;
        self.apply_discovery_report(probed);
        if self.best_discovery_candidate().is_some() {
            self.apply_reconnect_action(PcReconnectAction::UseBestDiscoveredRoute);
            Ok(())
        } else {
            Err(self
                .last_error
                .clone()
                .unwrap_or_else(|| "PC Host did not respond at that URL".to_string()))
        }
    }

    pub fn apply_discovery_report(&mut self, report: PcGatewayDiscoveryReport) {
        let has_online = report
            .candidates
            .iter()
            .any(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online);
        let has_candidates = !report.endpoint_candidates().is_empty();
        if let Some(candidate) = report
            .candidates
            .iter()
            .find(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online)
        {
            let endpoint = endpoint_health_from_candidate(candidate);
            self.active_endpoint = Some(endpoint.clone());
            upsert_endpoint_health(&mut self.endpoint_health, endpoint);
        }
        self.discovery_report = Some(report);
        if has_online {
            self.mark_online();
        } else if has_candidates {
            self.mark_waiting_for_pc();
        } else {
            self.mark_offline();
        }
    }

    pub fn apply_endpoint_health(
        &mut self,
        active: Option<PcGatewayEndpointHealth>,
        all: Vec<PcGatewayEndpointHealth>,
    ) {
        self.active_endpoint = active;
        self.endpoint_health = all;
        if self.active_endpoint.is_some() {
            self.mark_online();
        } else if self
            .endpoint_health
            .iter()
            .any(|endpoint| endpoint.failure_count > 0)
        {
            self.mark_offline();
        }
    }

    pub fn active_workspace_connection(&self) -> Option<WorkspaceConnection> {
        let request = self.request.as_ref()?;
        let active_endpoint = self.active_endpoint.as_ref()?;
        let primary_candidate = self
            .active_discovery_candidate()
            .unwrap_or_else(|| inferred_endpoint_candidate(active_endpoint));

        let mut gateway = PcGatewayConfig::new(
            request.gateway_id.clone(),
            request.gateway_label.clone(),
            primary_candidate.base_url.clone(),
            request.device_id.clone(),
        );
        gateway.auth_token =
            (!request.auth_token.trim().is_empty()).then(|| request.auth_token.clone());
        gateway.trust_level = PcGatewayTrustLevel::PairedCommandExecution;
        gateway.transport_mode = primary_candidate.transport_mode.clone();
        gateway.allow_http_on_local_network = primary_candidate.transport_mode.is_local_only();
        if let Some(report) = self.discovery_report.as_ref() {
            gateway.endpoint_candidates = report.endpoint_candidates();
        }

        Some(
            WorkspaceConnection::pc_gateway(
                format!("pc:{}:{}", request.gateway_id, request.workspace_id),
                request.gateway_label.clone(),
                request.workspace_id.clone(),
                request.workspace_id.clone(),
                request.workspace_root.clone(),
                gateway,
            )
            .with_status(WorkspaceConnectionStatus::Online),
        )
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.last_error = Some(message.clone());
        self.status = PcPairingUiStatus::Error(message);
    }

    pub fn primary_action_label(&self) -> &'static str {
        match self.status {
            PcPairingUiStatus::NotConfigured => "Configure PC pairing",
            PcPairingUiStatus::ReadyToExport => "Create PC pairing ZIP",
            PcPairingUiStatus::Exported => "Share pairing ZIP",
            PcPairingUiStatus::WaitingForPc => "Check PC connection",
            PcPairingUiStatus::Online => "Open PC workspace",
            PcPairingUiStatus::Offline => "Retry PC connection",
            PcPairingUiStatus::Error(_) => "Fix and retry",
        }
    }

    pub fn status_text(&self) -> String {
        match &self.status {
            PcPairingUiStatus::NotConfigured => "PC is not configured".to_string(),
            PcPairingUiStatus::ReadyToExport => "Ready to create PC pairing ZIP".to_string(),
            PcPairingUiStatus::Exported => match self.export.as_ref() {
                Some(export) => format!("Pairing ZIP created: {}", export.zip_path.display()),
                None => "Pairing ZIP created".to_string(),
            },
            PcPairingUiStatus::WaitingForPc => "Waiting for PC host to come online".to_string(),
            PcPairingUiStatus::Online => match self.active_endpoint.as_ref() {
                Some(endpoint) => format!("PC host is online via {}", endpoint.label),
                None => "PC host is online".to_string(),
            },
            PcPairingUiStatus::Offline => "PC host is offline".to_string(),
            PcPairingUiStatus::Error(message) => format!("PC pairing error: {}", message),
        }
    }

    pub fn active_route_text(&self) -> String {
        match self.active_endpoint.as_ref() {
            Some(endpoint) => format!(
                "{}\n{}\nlatency: {}\nsuccess: {} | failure: {}",
                endpoint.label,
                endpoint.base_url,
                format_latency(endpoint.last_latency_ms),
                endpoint.success_count,
                endpoint.failure_count
            ),
            None => {
                "No active PC route yet. Run a connection check or execute a PC workspace request."
                    .to_string()
            }
        }
    }

    pub fn endpoint_health_rows(&self) -> Vec<String> {
        if self.endpoint_health.is_empty() {
            return vec!["No endpoint health samples yet.".to_string()];
        }

        self.endpoint_health
            .iter()
            .map(|endpoint| {
                let error = endpoint
                    .last_error
                    .as_ref()
                    .map(|error| format!(" | last error: {}", error))
                    .unwrap_or_default();
                format!(
                    "{} — {} — score {} — {} — ok {} / fail {}{}",
                    endpoint.label,
                    endpoint.base_url,
                    endpoint.score(),
                    format_latency(endpoint.last_latency_ms),
                    endpoint.success_count,
                    endpoint.failure_count,
                    error
                )
            })
            .collect()
    }

    pub fn discovery_rows(&self) -> Vec<String> {
        let Some(report) = self.discovery_report.as_ref() else {
            return vec!["No discovery scan has been imported yet.".to_string()];
        };
        if report.candidates.is_empty() {
            return vec!["Discovery scan returned no PC gateway candidates.".to_string()];
        }
        report
            .candidates
            .iter()
            .map(|candidate| {
                let latency = candidate
                    .latency_ms
                    .map(|value| format!("{} ms", value))
                    .unwrap_or_else(|| "not probed".to_string());
                let message = candidate
                    .message
                    .as_ref()
                    .map(|message| format!(" | {}", message))
                    .unwrap_or_default();
                format!(
                    "{:?} — {:?} — {} — {}{}",
                    candidate.source,
                    candidate.status,
                    candidate.endpoint.base_url,
                    latency,
                    message
                )
            })
            .collect()
    }

    pub fn reconnect_controls(&self) -> Vec<PcReconnectControl> {
        vec![
            PcReconnectControl {
                action: PcReconnectAction::ScanAgain,
                label: "Scan again",
                description: "Start Android NSD/mDNS discovery again and refresh local PC candidates.",
                enabled: !matches!(self.status, PcPairingUiStatus::NotConfigured),
            },
            PcReconnectControl {
                action: PcReconnectAction::RetryActiveRoute,
                label: "Retry active route",
                description: "Keep the current endpoint but mark it for a fresh health check / reconnect attempt.",
                enabled: self.active_endpoint.is_some(),
            },
            PcReconnectControl {
                action: PcReconnectAction::UseBestDiscoveredRoute,
                label: "Use best discovered route",
                description: "Promote the best discovered online/found candidate as the active route.",
                enabled: self.best_discovery_candidate().is_some(),
            },
            PcReconnectControl {
                action: PcReconnectAction::ForgetBadRoutes,
                label: "Forget bad routes",
                description: "Clear failed health samples and rejected/probe-failed discovery candidates.",
                enabled: self.has_bad_routes(),
            },
        ]
    }

    /// Returns host detail from the discovery candidate health, if available.
    pub fn host_detail_text(&self) -> Option<String> {
        let report = self.discovery_report.as_ref()?;
        let online = report.candidates.iter().find(|c| c.health.is_some())?;
        let h = online.health.as_ref()?;
        let mut lines = vec![
            format!("Gateway: {}", h.gateway_id),
            format!("Version: {}", h.version),
        ];
        if h.uptime_secs > 0 {
            let hours = h.uptime_secs / 3600;
            let mins = (h.uptime_secs % 3600) / 60;
            lines.push(format!("Uptime: {}h {}m", hours, mins));
        }
        lines.push(format!(
            "Requests: {} ({} errors)",
            h.request_count, h.error_count
        ));
        Some(lines.join("\n"))
    }

    pub fn apply_reconnect_action(&mut self, action: PcReconnectAction) -> PcReconnectEffect {
        self.last_reconnect_action = Some(action.clone());
        self.reconnect_generation = self.reconnect_generation.saturating_add(1);
        match action {
            PcReconnectAction::ScanAgain => {
                self.mark_waiting_for_pc();
                PcReconnectEffect::StartDiscovery {
                    request_id: format!("pc-scan-{}", self.reconnect_generation),
                }
            }
            PcReconnectAction::RetryActiveRoute => match self.active_endpoint.as_ref() {
                Some(endpoint) => {
                    let base_url = endpoint.base_url.clone();
                    self.mark_waiting_for_pc();
                    PcReconnectEffect::RetryRoute { base_url }
                }
                None => PcReconnectEffect::None,
            },
            PcReconnectAction::UseBestDiscoveredRoute => match self.best_discovery_candidate() {
                Some(candidate) => {
                    let endpoint = PcGatewayEndpointHealth {
                        label: candidate.endpoint.label.clone(),
                        base_url: candidate.endpoint.base_url.clone(),
                        success_count: if candidate.status == PcGatewayDiscoveryStatus::Online {
                            1
                        } else {
                            0
                        },
                        failure_count: 0,
                        last_latency_ms: candidate.latency_ms,
                        last_error: None,
                    };
                    let base_url = endpoint.base_url.clone();
                    self.active_endpoint = Some(endpoint.clone());
                    self.endpoint_health.insert(0, endpoint);
                    self.mark_online();
                    PcReconnectEffect::SelectedRoute { base_url }
                }
                None => PcReconnectEffect::None,
            },
            PcReconnectAction::ForgetBadRoutes => {
                self.endpoint_health
                    .retain(|endpoint| endpoint.last_error.is_none() && endpoint.is_healthy());
                if let Some(report) = self.discovery_report.as_mut() {
                    report.candidates.retain(|candidate| {
                        matches!(
                            candidate.status,
                            PcGatewayDiscoveryStatus::Found | PcGatewayDiscoveryStatus::Online
                        )
                    });
                }
                self.last_error = None;
                if self.active_endpoint.is_some() {
                    self.mark_online();
                } else if self
                    .discovery_report
                    .as_ref()
                    .map(|report| !report.endpoint_candidates().is_empty())
                    .unwrap_or(false)
                {
                    self.mark_waiting_for_pc();
                } else {
                    self.mark_offline();
                }
                PcReconnectEffect::None
            }
        }
    }

    fn best_discovery_candidate(&self) -> Option<PcGatewayDiscoveryCandidate> {
        let candidates = self.discovery_report.as_ref()?.candidates.clone();
        candidates
            .iter()
            .find(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online)
            .cloned()
            .or_else(|| {
                candidates
                    .into_iter()
                    .find(|candidate| candidate.status == PcGatewayDiscoveryStatus::Found)
            })
    }

    fn active_discovery_candidate(&self) -> Option<PcGatewayEndpointCandidate> {
        let active_base_url = self.active_endpoint.as_ref()?.base_url.as_str();
        self.discovery_report
            .as_ref()?
            .candidates
            .iter()
            .find(|candidate| candidate.endpoint.base_url == active_base_url)
            .map(|candidate| candidate.endpoint.clone())
    }

    fn has_bad_routes(&self) -> bool {
        self.endpoint_health
            .iter()
            .any(|endpoint| endpoint.last_error.is_some() || !endpoint.is_healthy())
            || self
                .discovery_report
                .as_ref()
                .map(|report| {
                    report.candidates.iter().any(|candidate| {
                        matches!(
                            candidate.status,
                            PcGatewayDiscoveryStatus::Rejected
                                | PcGatewayDiscoveryStatus::ProbeFailed
                        )
                    })
                })
                .unwrap_or(false)
    }
}

fn format_latency(latency_ms: Option<u128>) -> String {
    latency_ms
        .map(|latency| format!("{} ms", latency))
        .unwrap_or_else(|| "not measured".to_string())
}

#[cfg(test)]
mod tests {
    use super::{PcPairingUiState, PcPairingUiStatus, PcReconnectAction, PcReconnectEffect};
    use crate::pc_pairing_manager::MobilePcPairingRequest;
    use deepseek_mobile_core::{
        PcGatewayDiscoveryCandidate, PcGatewayDiscoveryReport, PcGatewayDiscoveryService,
        PcGatewayDiscoverySource, PcGatewayDiscoveryStatus, PcGatewayEndpointCandidate,
        PcGatewayEndpointHealth, PcGatewayTransportMode, WorkspaceBackendKind,
        WorkspaceConnectionStatus, DEFAULT_PC_GATEWAY_PORT,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn starts_not_configured() {
        let state = PcPairingUiState::default();
        assert_eq!(state.status, PcPairingUiStatus::NotConfigured);
        assert_eq!(state.primary_action_label(), "Configure PC pairing");
    }

    #[test]
    fn configure_moves_to_ready() {
        let mut state = PcPairingUiState::default();
        state.configure(sample_request());
        assert_eq!(state.status, PcPairingUiStatus::ReadyToExport);
        assert_eq!(state.primary_action_label(), "Create PC pairing ZIP");
    }

    #[test]
    fn export_zip_moves_to_exported() {
        let mut state = PcPairingUiState::default();
        let output_dir = temp_dir();
        state.configure(sample_request());
        let zip_path = state.export_zip(&output_dir).unwrap();
        assert!(zip_path.exists());
        assert!(matches!(state.status, PcPairingUiStatus::Exported));
        assert!(state.status_text().contains("Pairing ZIP created"));
        let _ = fs::remove_dir_all(output_dir);
    }

    #[test]
    fn export_without_request_sets_error() {
        let mut state = PcPairingUiState::default();
        let result = state.export_zip(temp_dir());
        assert!(result.is_none());
        assert!(matches!(state.status, PcPairingUiStatus::Error(_)));
    }

    #[test]
    fn connection_status_transitions() {
        let mut state = PcPairingUiState::default();
        state.mark_waiting_for_pc();
        assert_eq!(state.status, PcPairingUiStatus::WaitingForPc);
        state.mark_online();
        assert_eq!(state.status, PcPairingUiStatus::Online);
        state.mark_offline();
        assert_eq!(state.status, PcPairingUiStatus::Offline);
    }

    #[test]
    fn endpoint_health_marks_online_and_describes_route() {
        let mut state = PcPairingUiState::default();
        let endpoint = PcGatewayEndpointHealth {
            label: "same-lan".to_string(),
            base_url: "http://192.168.1.10:8787".to_string(),
            success_count: 2,
            failure_count: 0,
            last_latency_ms: Some(42),
            last_error: None,
        };
        state.apply_endpoint_health(Some(endpoint.clone()), vec![endpoint]);
        assert_eq!(state.status, PcPairingUiStatus::Online);
        assert!(state.status_text().contains("same-lan"));
        assert!(state.active_route_text().contains("42 ms"));
        assert_eq!(state.endpoint_health_rows().len(), 1);
    }

    #[test]
    fn discovery_report_is_visible_in_rows() {
        let mut state = PcPairingUiState::default();
        let report = PcGatewayDiscoveryService::default()
            .from_manual_hosts(vec!["192.168.1.10".to_string()], DEFAULT_PC_GATEWAY_PORT);
        state.apply_discovery_report(report);
        assert_eq!(state.status, PcPairingUiStatus::WaitingForPc);
        assert!(state.discovery_rows()[0].contains("192.168.1.10"));
    }

    #[test]
    fn online_discovery_promotes_active_route() {
        let mut state = PcPairingUiState::default();
        state.configure(sample_request());
        state.apply_discovery_report(online_report());

        assert_eq!(state.status, PcPairingUiStatus::Online);
        assert_eq!(
            state
                .active_endpoint
                .as_ref()
                .map(|endpoint| endpoint.base_url.as_str()),
            Some("http://192.168.1.10:8787")
        );
        assert_eq!(state.endpoint_health.len(), 1);
    }

    #[test]
    fn active_route_builds_persistable_pc_workspace_connection() {
        let mut state = PcPairingUiState::default();
        state.configure(sample_request());
        state.apply_discovery_report(online_report());

        let connection = state.active_workspace_connection().unwrap();

        assert_eq!(connection.backend, WorkspaceBackendKind::PcGateway);
        assert_eq!(connection.status, WorkspaceConnectionStatus::Online);
        assert_eq!(connection.workspace_id, "local");
        let gateway = connection.pc_gateway.unwrap();
        assert_eq!(gateway.base_url, "http://192.168.1.10:8787");
        assert_eq!(gateway.auth_token.as_deref(), Some("pairing-token"));
        assert_eq!(gateway.endpoint_candidates.len(), 1);
    }

    #[test]
    fn scan_again_returns_discovery_effect() {
        let mut state = PcPairingUiState::default();
        state.configure(sample_request());
        let effect = state.apply_reconnect_action(PcReconnectAction::ScanAgain);
        assert!(matches!(effect, PcReconnectEffect::StartDiscovery { .. }));
        assert_eq!(state.status, PcPairingUiStatus::WaitingForPc);
    }

    #[test]
    fn use_best_discovered_route_promotes_candidate() {
        let mut state = PcPairingUiState::default();
        let report = PcGatewayDiscoveryService::default()
            .from_manual_hosts(vec!["192.168.1.10".to_string()], DEFAULT_PC_GATEWAY_PORT);
        state.apply_discovery_report(report);
        let effect = state.apply_reconnect_action(PcReconnectAction::UseBestDiscoveredRoute);
        assert!(matches!(effect, PcReconnectEffect::SelectedRoute { .. }));
        assert_eq!(state.status, PcPairingUiStatus::Online);
        assert!(state.active_endpoint.is_some());
    }

    #[test]
    fn forget_bad_routes_removes_failed_health() {
        let mut state = PcPairingUiState::default();
        state.endpoint_health.push(PcGatewayEndpointHealth {
            label: "bad".to_string(),
            base_url: "http://192.168.1.10:8787".to_string(),
            success_count: 0,
            failure_count: 3,
            last_latency_ms: Some(500),
            last_error: Some("timeout".to_string()),
        });
        assert!(state
            .reconnect_controls()
            .iter()
            .any(
                |control| control.action == PcReconnectAction::ForgetBadRoutes && control.enabled
            ));
        let _ = state.apply_reconnect_action(PcReconnectAction::ForgetBadRoutes);
        assert!(state.endpoint_health.is_empty());
    }

    fn sample_request() -> MobilePcPairingRequest {
        MobilePcPairingRequest::new(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "pairing-token",
        )
    }

    fn online_report() -> PcGatewayDiscoveryReport {
        PcGatewayDiscoveryReport {
            service_name: "test".to_string(),
            candidates: vec![PcGatewayDiscoveryCandidate {
                source: PcGatewayDiscoverySource::Mdns,
                endpoint: PcGatewayEndpointCandidate::new(
                    "mdns:laptop",
                    "http://192.168.1.10:8787",
                    PcGatewayTransportMode::LocalNetworkHttp,
                ),
                status: PcGatewayDiscoveryStatus::Online,
                message: None,
                latency_ms: Some(12),
                health: None,
            }],
        }
    }

    fn temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-mobile-pairing-state-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}

fn endpoint_health_from_candidate(
    candidate: &PcGatewayDiscoveryCandidate,
) -> PcGatewayEndpointHealth {
    PcGatewayEndpointHealth {
        label: candidate.endpoint.label.clone(),
        base_url: candidate.endpoint.base_url.clone(),
        success_count: if candidate.status == PcGatewayDiscoveryStatus::Online {
            1
        } else {
            0
        },
        failure_count: 0,
        last_latency_ms: candidate.latency_ms,
        last_error: None,
    }
}

fn upsert_endpoint_health(
    rows: &mut Vec<PcGatewayEndpointHealth>,
    endpoint: PcGatewayEndpointHealth,
) {
    if let Some(existing) = rows
        .iter_mut()
        .find(|row| row.base_url == endpoint.base_url)
    {
        *existing = endpoint;
    } else {
        rows.insert(0, endpoint);
    }
}

fn inferred_endpoint_candidate(endpoint: &PcGatewayEndpointHealth) -> PcGatewayEndpointCandidate {
    let transport_mode = if endpoint.base_url.starts_with("https://") {
        PcGatewayTransportMode::LocalNetworkHttps
    } else {
        PcGatewayTransportMode::LocalNetworkHttp
    };
    PcGatewayEndpointCandidate::new(
        endpoint.label.clone(),
        endpoint.base_url.clone(),
        transport_mode,
    )
}
