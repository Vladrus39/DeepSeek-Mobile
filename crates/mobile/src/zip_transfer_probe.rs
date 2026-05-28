//! Headless ZIP export/import probe for ADB E2E.
//!
//! Triggered by creating `files/deepseek-mobile/.zip_transfer_probe_requested` in app data dir.
//! Writes a single-line result into `files/deepseek-mobile/.zip_transfer_probe_result`.

use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::NativeMobileEvent;
use crate::project_transfer_state::{default_export_dir, default_phone_workspace_root, ProjectTransferState};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const REQUEST_FLAG: &str = ".zip_transfer_probe_requested";
const RESULT_FILE: &str = ".zip_transfer_probe_result";

fn flag_path() -> PathBuf {
    default_data_dir().join(REQUEST_FLAG)
}

fn result_path() -> PathBuf {
    default_data_dir().join(RESULT_FILE)
}

pub fn is_probe_requested() -> bool {
    flag_path().exists()
}

fn write_result(line: &str) {
    let path = result_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{line}\n"));
}

fn ensure_minimal_workspace(root: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| format!("workspace mkdir failed: {e}"))?;
    let readme = root.join("README.md");
    if !readme.exists() {
        fs::write(&readme, "# DeepSeek Mobile E2E\n").map_err(|e| format!("write README failed: {e}"))?;
    }
    Ok(())
}

async fn wait_for_share_callback(timeout: Duration) -> Result<&'static str, String> {
    let start = Instant::now();
    loop {
        let bridge = crate::native_host_runtime::snapshot();
        match bridge.last_event {
            Some(NativeMobileEvent::FileShared) => return Ok("file_shared"),
            Some(NativeMobileEvent::ShareFailed(message)) => return Err(message),
            _ => {}
        }
        if start.elapsed() >= timeout {
            return Err("timeout waiting for share callback".to_string());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Runs one ZIP export + share when [REQUEST_FLAG] exists.
pub async fn run_if_requested() {
    if !is_probe_requested() {
        return;
    }
    let _ = fs::remove_file(flag_path());

    let workspace_root = default_phone_workspace_root();
    let export_dir = default_export_dir();
    if let Err(err) = ensure_minimal_workspace(&workspace_root) {
        write_result(&format!("FAIL {err}"));
        return;
    }

    let mut transfer = ProjectTransferState::default();
    let report = match transfer.export_workspace(&workspace_root, &export_dir) {
        Ok(r) => r,
        Err(e) => {
            write_result(&format!("FAIL export {e}"));
            return;
        }
    };

    // Enqueue Android share.
    {
        let mut bridge = crate::native_host_runtime::snapshot();
        bridge.enqueue_share_file(report.archive_path.display().to_string());
        crate::native_host_runtime::replace(bridge);
    }

    // The Android coordinator confirms immediately after launching chooser.
    match wait_for_share_callback(Duration::from_secs(15)).await {
        Ok(kind) => write_result(&format!("PASS export={} share={kind}", report.archive_path.display())),
        Err(error) => write_result(&format!(
            "FAIL export={} share_error={}",
            report.archive_path.display(),
            error.replace('\n', " ")
        )),
    }
}

