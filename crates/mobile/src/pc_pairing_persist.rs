//! Persist PC pairing UI state between app launches.

use crate::mobile_runtime_config::default_data_dir;
use crate::pc_pairing_manager::MobilePcPairingRequest;
use crate::pc_pairing_state::{PcPairingUiState, PcPairingUiStatus};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedPcPairing {
    request: Option<MobilePcPairingRequest>,
    status: String,
    export_zip: Option<String>,
}

fn persist_path() -> PathBuf {
    default_data_dir().join("pc_pairing_ui.json")
}

pub fn load_persisted_pairing() -> Option<PcPairingUiState> {
    let path = persist_path();
    let raw = fs::read_to_string(&path).ok()?;
    let saved: PersistedPcPairing = serde_json::from_str(&raw).ok()?;
    let status = match saved.status.as_str() {
        "ready" => PcPairingUiStatus::ReadyToExport,
        "exported" => PcPairingUiStatus::Exported,
        "waiting" => PcPairingUiStatus::WaitingForPc,
        "online" => PcPairingUiStatus::Online,
        "offline" => PcPairingUiStatus::Offline,
        other if other.starts_with("error:") => {
            PcPairingUiStatus::Error(other.trim_start_matches("error:").to_string())
        }
        _ => PcPairingUiStatus::NotConfigured,
    };
    Some(PcPairingUiState {
        status,
        request: saved.request,
        export: None,
        discovery_report: None,
        active_endpoint: None,
        endpoint_health: Vec::new(),
        reconnect_generation: 0,
        last_reconnect_action: None,
        last_error: None,
    })
}

pub fn save_pairing(state: &PcPairingUiState) {
    let status = match &state.status {
        PcPairingUiStatus::NotConfigured => "not_configured",
        PcPairingUiStatus::ReadyToExport => "ready",
        PcPairingUiStatus::Exported => "exported",
        PcPairingUiStatus::WaitingForPc => "waiting",
        PcPairingUiStatus::Online => "online",
        PcPairingUiStatus::Offline => "offline",
        PcPairingUiStatus::Error(message) => {
            let _ = message;
            "error"
        }
    };
    let payload = PersistedPcPairing {
        request: state.request.clone(),
        status: status.to_string(),
        export_zip: state
            .export
            .as_ref()
            .map(|export| export.zip_path.display().to_string()),
    };
    let path = persist_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(path, json);
    }
}
