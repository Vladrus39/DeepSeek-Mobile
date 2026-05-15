//! Mobile-safe file operation tools.

use super::{required_str, optional_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::workspace_files::WorkspaceFileService;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct ListDirTool;
pub struct EditFileTool;

impl ToolSpec for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read a UTF-8 text file from the active workspace or an approved external folder."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" }
            },
            "required": ["path"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = required_str(&input, "path")?;
        let service = WorkspaceFileService::new(context.workspace.clone());
        let content = service.read_text_file(path)?;
        Ok(ToolResult::success(content))
    }
}

impl ToolSpec for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write UTF-8 content to a file. File writes require approval outside YOLO/auto-approve mode."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
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
        ApprovalRequirement::Suggest
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = required_str(&input, "path")?;
        let content = required_str(&input, "content")?;
        let service = WorkspaceFileService::new(context.workspace.clone());
        service.write_text_file(path, content)?;
        Ok(ToolResult::success(format!(
            "Wrote {} bytes to {}",
            content.len(),
            path
        )))
    }
}

impl ToolSpec for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List files and directories inside the active workspace."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path, defaults to ." }
            }
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = optional_str(&input, "path").unwrap_or(".");
        let service = WorkspaceFileService::new(context.workspace.clone());
        let entries = service.list_files(path)?;
        let value = serde_json::to_value(entries)?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&value)?).with_metadata(value))
    }
}

impl ToolSpec for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replace exact text in a UTF-8 file. Requires approval because it modifies files."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" },
                "search": { "type": "string", "description": "Exact text to find" },
                "replace": { "type": "string", "description": "Replacement text" }
            },
            "required": ["path", "search", "replace"]
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
        ApprovalRequirement::Suggest
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = required_str(&input, "path")?;
        let search = required_str(&input, "search")?;
        let replace = required_str(&input, "replace")?;

        if search.is_empty() {
            return Err(anyhow!("search text must not be empty"));
        }

        let service = WorkspaceFileService::new(context.workspace.clone());
        let content = service.read_text_file(path)?;
        let count = content.matches(search).count();
        if count == 0 {
            return Err(anyhow!("search text not found in {}", path));
        }

        let updated = content.replace(search, replace);
        service.write_text_file(path, &updated)?;

        Ok(ToolResult::success(format!(
            "Replaced {} occurrence(s) in {}",
            count, path
        )))
    }
}

/// Compatibility wrapper kept for older code paths while the agent migrates to
/// separate read/write/edit/list tools.
pub struct FileOpsTool;

impl ToolSpec for FileOpsTool {
    fn name(&self) -> &str {
        "file_ops"
    }

    fn description(&self) -> &str {
        "Legacy compatibility wrapper. Prefer read_file, write_file, edit_file and list_dir."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string" },
                "path": { "type": "string" },
                "content": { "type": "string" },
                "search": { "type": "string" },
                "replace": { "type": "string" }
            },
            "required": ["operation"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::RequiresApproval, ToolCapability::Sandboxable]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let operation = required_str(&input, "operation")?;
        match operation {
            "read" | "read_file" => ReadFileTool.execute(input, context),
            "write" | "write_file" => WriteFileTool.execute(input, context),
            "edit" | "edit_file" => EditFileTool.execute(input, context),
            "list" | "list_dir" => ListDirTool.execute(input, context),
            other => Err(anyhow!("unsupported file operation: {}", other)),
        }
    }
}
