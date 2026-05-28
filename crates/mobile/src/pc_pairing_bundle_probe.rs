//! Headless PC pairing ZIP export (ADB E2E): same path as the PC Host UI export.

use crate::mobile_runtime_config::default_data_dir;
use crate::pc_pairing_manager::{MobilePcPairingRequest, PcPairingManager};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const REQUEST_FLAG: &str = ".pc_pairing_bundle_probe_requested";
const RESULT_FILE: &str = ".pc_pairing_bundle_probe_result";

fn flag_path() -> PathBuf {
    default_data_dir().join(REQUEST_FLAG)
}

fn result_path() -> PathBuf {
    default_data_dir().join(RESULT_FILE)
}

pub fn is_probe_requested() -> bool {
    flag_path().exists()
}

fn clear_request_flag() {
    let _ = fs::remove_file(flag_path());
}

fn write_result(line: &str) {
    if let Some(parent) = result_path().parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(result_path(), format!("{line}\n"));
}

pub fn run_if_requested() {
    if !is_probe_requested() {
        return;
    }
    clear_request_flag();

    let output_dir = default_data_dir().join("pairing-export");
    if let Err(error) = fs::create_dir_all(&output_dir) {
        write_result(&format!("FAIL mkdir {}: {error}", output_dir.display()));
        return;
    }

    let request = MobilePcPairingRequest::new(
        "mobile-pairing-e2e",
        "DeepSeek PC (pairing bundle)",
        "mobile-device",
        "DeepSeek Mobile",
        "default",
        ".",
        Uuid::new_v4().simple().to_string(),
    );

    match PcPairingManager::export_zip(request, &output_dir) {
        Ok(export) => {
            write_result(&format!(
                "PASS zip={} gateway_id={} bind={}",
                export.zip_path.display(),
                export.gateway_id,
                export.bind_addr,
            ));
        }
        Err(error) => {
            write_result(&format!("FAIL export: {error}"));
        }
    }
}
