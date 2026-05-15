use crate::pc_pairing_manager::{MobilePcPairingExport, MobilePcPairingRequest, PcPairingManager};
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
pub struct PcPairingUiState {
    pub status: PcPairingUiStatus,
    pub request: Option<MobilePcPairingRequest>,
    pub export: Option<MobilePcPairingExport>,
    pub last_error: Option<String>,
}

impl Default for PcPairingUiState {
    fn default() -> Self {
        Self {
            status: PcPairingUiStatus::NotConfigured,
            request: None,
            export: None,
            last_error: None,
        }
    }
}

impl PcPairingUiState {
    pub fn configure(&mut self, request: MobilePcPairingRequest) {
        self.request = Some(request);
        self.export = None;
        self.last_error = None;
        self.status = PcPairingUiStatus::ReadyToExport;
    }

    pub fn export_zip(&mut self, output_dir: impl AsRef<Path>) -> Option<PathBuf> {
        let Some(request) = self.request.clone() else {
            self.set_error("PC pairing request is not configured");
            return None;
        };

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
            PcPairingUiStatus::Online => "PC host is online".to_string(),
            PcPairingUiStatus::Offline => "PC host is offline".to_string(),
            PcPairingUiStatus::Error(message) => format!("PC pairing error: {}", message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PcPairingUiState, PcPairingUiStatus};
    use crate::pc_pairing_manager::MobilePcPairingRequest;
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

    fn sample_request() -> MobilePcPairingRequest {
        MobilePcPairingRequest::new(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "secret-token",
        )
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
