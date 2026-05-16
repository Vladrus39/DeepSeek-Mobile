//! Model-visible workspace snapshot tools.

use super::{optional_str, required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::snapshots::WorkspaceSnapshotService;
use anyhow::Result;
use serde_json::{json, Value};

pub struct CreateSnapshotTool;
pub struct ListSnapshotsTool;
pub struct RestoreSnapshotTool;

impl ToolSpec for CreateSnapshotTool {
    fn name(&self) -> &str {
        "snapshot_create"
    }

    fn description(&self) -> &str {
        "Create a portable workspace snapshot before risky edits or commands."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "reason": { "type": "string", "description": "Why the snapshot is being created" },
                "store_root": { "type": "string", "description": "Optional snapshot store root. Defaults to .deepseek-mobile/snapshots inside workspace." }
            }
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let reason = optional_str(&input, "reason").unwrap_or("manual snapshot");
        let service = snapshot_service(input, context)?;
        let snapshot = service.create_snapshot(reason)?;
        Ok(ToolResult::success(format!(
            "Created snapshot {} with {} file(s), {} bytes",
            snapshot.id, snapshot.file_count, snapshot.total_bytes
        ))
        .with_metadata(serde_json::to_value(snapshot)?))
    }
}

impl ToolSpec for ListSnapshotsTool {
    fn name(&self) -> &str {
        "snapshot_list"
    }

    fn description(&self) -> &str {
        "List available workspace snapshots for rollback."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "store_root": { "type": "string", "description": "Optional snapshot store root. Defaults to .deepseek-mobile/snapshots inside workspace." }
            }
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let service = snapshot_service(input, context)?;
        let snapshots = service.list_snapshots()?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&snapshots)?)
            .with_metadata(serde_json::to_value(snapshots)?))
    }
}

impl ToolSpec for RestoreSnapshotTool {
    fn name(&self) -> &str {
        "snapshot_restore"
    }

    fn description(&self) -> &str {
        "Restore a workspace snapshot. This can overwrite and delete files, so it requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "snapshot_id": { "type": "string", "description": "Snapshot id returned by snapshot_create or snapshot_list" },
                "store_root": { "type": "string", "description": "Optional snapshot store root. Defaults to .deepseek-mobile/snapshots inside workspace." }
            },
            "required": ["snapshot_id"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::WritesFiles,
            ToolCapability::RequiresApproval,
            ToolCapability::Sandboxable,
        ]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let snapshot_id = required_str(&input, "snapshot_id")?;
        let service = snapshot_service(input, context)?;
        let report = service.restore_snapshot(snapshot_id)?;
        Ok(ToolResult::success(format!(
            "Restored snapshot {}: restored {}, removed {}, skipped {}",
            report.snapshot_id, report.restored_files, report.removed_files, report.skipped_files
        ))
        .with_metadata(serde_json::to_value(report)?))
    }
}

fn snapshot_service(input: Value, context: &ToolContext) -> Result<WorkspaceSnapshotService> {
    let store_root = optional_str(&input, "store_root")
        .map(|path| context.resolve_path(path))
        .transpose()?
        .unwrap_or_else(|| context.workspace.root.join(".deepseek-mobile").join("snapshots"));
    Ok(WorkspaceSnapshotService::new(context.workspace.clone(), store_root))
}

#[cfg(test)]
mod tests {
    use super::{CreateSnapshotTool, ListSnapshotsTool, RestoreSnapshotTool};
    use crate::tools::{ToolContext, ToolSpec};
    use crate::workspace::{ExecutorKind, Workspace};
    use serde_json::json;
    use std::fs;

    fn context(name: &str) -> (ToolContext, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_snapshot_tool_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::new("w1", "Workspace", root.clone(), ExecutorKind::LocalAndroid);
        (ToolContext::new(workspace), root)
    }

    #[test]
    fn creates_lists_and_restores_snapshot() {
        let (context, root) = context("roundtrip");
        fs::write(root.join("README.md"), "v1").unwrap();

        let create = CreateSnapshotTool.execute(json!({"reason":"test"}), &context).unwrap();
        let snapshot_id = create.metadata.unwrap()["id"].as_str().unwrap().to_string();
        fs::write(root.join("README.md"), "broken").unwrap();

        let list = ListSnapshotsTool.execute(json!({}), &context).unwrap();
        assert!(list.content.contains(&snapshot_id));

        RestoreSnapshotTool
            .execute(json!({"snapshot_id": snapshot_id}), &context)
            .unwrap();
        assert_eq!(fs::read_to_string(root.join("README.md")).unwrap(), "v1");

        let _ = fs::remove_dir_all(root);
    }
}