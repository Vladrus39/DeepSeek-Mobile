//! Grant filesystem access when the user approves a tool call in chat.

use crate::config::ExternalAccessMode;
use crate::pc_gateway_client::PcGatewayClient;
use crate::tool_call::ToolCallRequest;
use crate::tools::ToolContext;
use crate::workspace::ExecutorKind;
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

const PATH_TOOLS: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "delete_file",
    "list_dir",
    "open_path",
    "apply_patch",
];

pub fn extract_path_argument(call: &ToolCallRequest) -> Option<String> {
    if !PATH_TOOLS.contains(&call.name.as_str()) {
        return None;
    }
    let path = call.arguments.get("path")?.as_str()?;
    Some(path.to_string())
}

pub fn extract_paths_from_patch(arguments: &Value) -> Vec<String> {
    let Some(ops) = arguments.get("operations").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    ops.iter()
        .filter_map(|op| op.get("path").and_then(|v| v.as_str()).map(str::to_string))
        .collect()
}

/// After user approval, allow the target path(s) on PC gateway or local trusted list.
pub async fn grant_paths_for_approved_call(
    call: &ToolCallRequest,
    context: &mut ToolContext,
    pc_gateway: Option<&PcGatewayClient>,
) -> Result<()> {
    let mut paths = Vec::new();
    if let Some(path) = extract_path_argument(call) {
        paths.push(path);
    }
    if call.name == "apply_patch" {
        paths.extend(extract_paths_from_patch(&call.arguments));
    }
    if paths.is_empty() {
        return Ok(());
    }

    if matches!(context.workspace.executor, ExecutorKind::PcGateway) {
        let Some(client) = pc_gateway else {
            return Ok(());
        };
        let workspace_id = context.workspace.id.clone();
        for path in paths {
            let _ = client
                .grant_trusted_path(workspace_id.clone(), path)
                .await?;
        }
        return Ok(());
    }

    for path in paths {
        grant_local_trusted_path(context, &path)?;
    }
    context.external_access = ExternalAccessMode::AllowedByUserGrant;
    Ok(())
}

fn grant_local_trusted_path(context: &mut ToolContext, raw_path: &str) -> Result<()> {
    let candidate = PathBuf::from(raw_path);
    let grant_root = if candidate.is_absolute() {
        if candidate.is_dir() {
            candidate
        } else {
            candidate
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| context.workspace.root.clone())
        }
    } else if let Some(resolved) = context.workspace.resolve_relative_path(raw_path) {
        resolved
    } else {
        return Ok(());
    };
    let canonical = grant_root.canonicalize().unwrap_or(grant_root);
    if canonical.starts_with(&context.workspace.root) {
        return Ok(());
    }
    if context
        .trusted_external_paths
        .iter()
        .any(|trusted| canonical.starts_with(trusted))
    {
        return Ok(());
    }
    context.trusted_external_paths.push(canonical);
    Ok(())
}

pub fn path_outside_workspace(context: &ToolContext, raw_path: &str) -> bool {
    let candidate = PathBuf::from(raw_path);
    if candidate.is_absolute() {
        return !candidate.starts_with(&context.workspace.root);
    }
    context.workspace.resolve_relative_path(raw_path).is_none()
}
