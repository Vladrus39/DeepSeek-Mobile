//! Direct tool smoke test on the app sandbox workspace (no LLM).
//!
//! Trigger: `files/deepseek-mobile/.tools_smoke_probe_requested`
//! Result:  `files/deepseek-mobile/.tools_smoke_probe_result`

use crate::mobile_runtime_config::default_data_dir;
use deepseek_mobile_core::tools::{default_mobile_tool_registry, ToolContext};
use deepseek_mobile_core::{ExecutorKind, Workspace};
use serde_json::json;
use std::fs;

const REQUEST_FLAG: &str = ".tools_smoke_probe_requested";
const RESULT_FILE: &str = ".tools_smoke_probe_result";

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

fn sandbox_workspace() -> Workspace {
    let root = default_data_dir().join("workspace");
    let _ = fs::create_dir_all(&root);
    Workspace::new(
        "phone-sandbox-smoke",
        "Phone sandbox",
        root,
        ExecutorKind::LocalAndroid,
    )
}

/// Run workspace_overview + apply_patch + read_file on LocalAndroid sandbox.
pub fn run_if_requested() {
    if !is_probe_requested() {
        return;
    }
    let _ = fs::remove_file(default_data_dir().join(REQUEST_FLAG));

    let registry = default_mobile_tool_registry();
    let workspace = sandbox_workspace();
    let context = ToolContext::new(workspace.clone());
    let mut steps: Vec<String> = Vec::new();

    match registry.execute("workspace_overview", json!({ "max_depth": 2 }), &context) {
        Ok(r) if r.success => steps.push("workspace_overview=ok".to_string()),
        Ok(r) => {
            write_result(&format!("FAIL workspace_overview {}", r.content));
            return;
        }
        Err(e) => {
            write_result(&format!("FAIL workspace_overview {e}"));
            return;
        }
    }

    let target = "smoke_patch_target.txt";
    let patch_args = json!({
        "operations": [{
            "type": "create",
            "path": target,
            "content": "SMOKE_PATCH_OK\n",
            "overwrite": true
        }]
    });
    match registry.execute("apply_patch", patch_args, &context) {
        Ok(r) if r.success => steps.push("apply_patch=ok".to_string()),
        Ok(r) => {
            write_result(&format!("FAIL apply_patch {}", r.content));
            return;
        }
        Err(e) => {
            write_result(&format!("FAIL apply_patch {e}"));
            return;
        }
    }

    let file_path = workspace.root.join(target);
    if !file_path.is_file() {
        write_result("FAIL apply_patch file missing on disk");
        return;
    }

    match registry.execute("read_file", json!({ "path": target }), &context) {
        Ok(r) if r.success && r.content.contains("SMOKE_PATCH_OK") => {
            steps.push("read_file=ok".to_string())
        }
        Ok(r) => {
            write_result(&format!("FAIL read_file {}", r.content));
            return;
        }
        Err(e) => {
            write_result(&format!("FAIL read_file {e}"));
            return;
        }
    }

    write_result(&format!(
        "PASS sandbox workspace={} {}",
        workspace.root.display(),
        steps.join(" ")
    ));
}
