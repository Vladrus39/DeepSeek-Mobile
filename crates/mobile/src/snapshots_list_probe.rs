//! Headless snapshot list probe (no LLM).
//!
//! Trigger: `files/deepseek-mobile/.snapshots_list_probe_requested`
//! Result:  `files/deepseek-mobile/.snapshots_list_probe_result`

use crate::mobile_runtime_config::{default_data_dir, MobileRuntimeConfig};
use deepseek_mobile_core::snapshots::WorkspaceSnapshotService;
use deepseek_mobile_core::{ExecutorKind, Workspace};
use std::fs;

const REQUEST_FLAG: &str = ".snapshots_list_probe_requested";
const RESULT_FILE: &str = ".snapshots_list_probe_result";

pub fn is_probe_requested() -> bool {
    default_data_dir().join(REQUEST_FLAG).exists()
}

fn write_result(line: &str) {
    let path = default_data_dir().join(RESULT_FILE);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{line}\n"));
}

pub fn run_if_requested() {
    if !is_probe_requested() {
        return;
    }
    let _ = fs::remove_file(default_data_dir().join(REQUEST_FLAG));

    let runtime = MobileRuntimeConfig::default_mobile();
    let workspace = Workspace::new(
        "mobile",
        "Mobile workspace",
        runtime.workspace_root.clone(),
        ExecutorKind::LocalAndroid,
    );
    let store_root = workspace.root.join(".deepseek-mobile").join("snapshots");

    match WorkspaceSnapshotService::new(workspace.clone(), store_root.clone()).list_snapshots() {
        Ok(snapshots) => write_result(&format!(
            "PASS snapshot_list count={} store={}",
            snapshots.len(),
            store_root.display()
        )),
        Err(error) => write_result(&format!("FAIL snapshot_list {error}")),
    }
}
