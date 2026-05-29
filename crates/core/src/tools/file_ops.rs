//! Mobile-safe file operation tools.

use super::{
    optional_str, required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult,
    ToolSpec,
};
use crate::workspace_files::{WorkspaceFileEntry, WorkspaceFileService};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::fs;

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
        let content = read_text_at(context, path)?;
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
        write_text_at(context, path, content)?;
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
        let entries = list_dir_at(context, path)?;
        let value = serde_json::to_value(&entries)?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&value)?).with_metadata(value))
    }
}

pub struct DeleteFileTool;

impl ToolSpec for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Permanently delete a file from the active workspace. Requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to delete" }
            },
            "required": ["path"]
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
        delete_file_at(context, path)?;
        Ok(ToolResult::success(format!("Deleted {}", path)))
    }
}

pub struct CopyFileTool;

impl ToolSpec for CopyFileTool {
    fn name(&self) -> &str {
        "copy_file"
    }

    fn description(&self) -> &str {
        "Copy a file from source to destination inside the active workspace. Requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "description": "Source file path" },
                "dest": { "type": "string", "description": "Destination file path" }
            },
            "required": ["source", "dest"]
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
        let source = required_str(&input, "source")?;
        let dest = required_str(&input, "dest")?;
        let service = WorkspaceFileService::new(context.workspace.clone());
        service.copy_file(source, dest)?;
        Ok(ToolResult::success(format!(
            "Copied {} to {}",
            source, dest
        )))
    }
}

pub struct MoveFileTool;

impl ToolSpec for MoveFileTool {
    fn name(&self) -> &str {
        "move_file"
    }

    fn description(&self) -> &str {
        "Move or rename a file from source to destination inside the active workspace. Requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "description": "Source file path" },
                "dest": { "type": "string", "description": "Destination file path" }
            },
            "required": ["source", "dest"]
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
        let source = required_str(&input, "source")?;
        let dest = required_str(&input, "dest")?;
        let service = WorkspaceFileService::new(context.workspace.clone());
        service.rename_file(source, dest)?;
        Ok(ToolResult::success(format!("Moved {} to {}", source, dest)))
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

        let content = read_text_at(context, path)?;
        let count = content.matches(search).count();
        if count == 0 {
            return Err(anyhow!("search text not found in {}", path));
        }

        let updated = content.replace(search, replace);
        write_text_at(context, path, &updated)?;

        Ok(ToolResult::success(format!(
            "Replaced {} occurrence(s) in {}",
            count, path
        )))
    }
}

pub struct ReadManyFilesTool;

impl ToolSpec for ReadManyFilesTool {
    fn name(&self) -> &str {
        "read_many_files"
    }

    fn description(&self) -> &str {
        "Read multiple files at once and return their contents concatenated with dividers. Files that don't exist or aren't readable are reported as errors per-path without failing the whole call."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of file paths to read"
                }
            },
            "required": ["paths"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let paths: Vec<String> = input["paths"]
            .as_array()
            .ok_or_else(|| anyhow!("paths must be an array of strings"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        if paths.is_empty() {
            return Err(anyhow!("paths must contain at least one path"));
        }

        let service = WorkspaceFileService::new(context.workspace.clone());
        let mut out = String::new();

        for path in &paths {
            out.push_str(&format!("\n===== {} =====\n", path));
            match service.read_text_file(path) {
                Ok(content) => {
                    out.push_str(&content);
                    if !content.ends_with('\n') {
                        out.push('\n');
                    }
                }
                Err(e) => {
                    out.push_str(&format!("[ERROR: {}]\n", e));
                }
            }
        }

        Ok(ToolResult::success(out))
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
        vec![
            ToolCapability::RequiresApproval,
            ToolCapability::Sandboxable,
        ]
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

fn read_text_at(context: &ToolContext, raw_path: &str) -> Result<String> {
    if let Ok(service) = workspace_service(context) {
        if let Ok(content) = service.read_text_file(raw_path) {
            return Ok(content);
        }
    }
    let path = context.resolve_path(raw_path)?;
    if !path.is_file() {
        return Err(anyhow!("path is not a file: {}", path.display()));
    }
    Ok(fs::read_to_string(path)?)
}

fn write_text_at(context: &ToolContext, raw_path: &str, content: &str) -> Result<()> {
    if let Ok(service) = workspace_service(context) {
        if service.write_text_file(raw_path, content).is_ok() {
            return Ok(());
        }
    }
    let path = context.resolve_path(raw_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn list_dir_at(context: &ToolContext, raw_path: &str) -> Result<Vec<WorkspaceFileEntry>> {
    if let Ok(service) = workspace_service(context) {
        if let Ok(entries) = service.list_files(raw_path) {
            return Ok(entries);
        }
    }
    let dir = context.resolve_path(raw_path)?;
    if !dir.is_dir() {
        return Err(anyhow!("path is not a directory: {}", dir.display()));
    }
    let workspace_root = &context.workspace.root;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let absolute_path = entry.path();
        let relative_path = absolute_path
            .strip_prefix(workspace_root)
            .unwrap_or(&absolute_path)
            .to_path_buf();
        entries.push(WorkspaceFileEntry {
            path: relative_path,
            is_dir: metadata.is_dir(),
            size_bytes: metadata.len(),
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn delete_file_at(context: &ToolContext, raw_path: &str) -> Result<()> {
    if let Ok(service) = workspace_service(context) {
        if service.delete_file(raw_path).is_ok() {
            return Ok(());
        }
    }
    let path = context.resolve_path(raw_path)?;
    if !path.is_file() {
        return Err(anyhow!("path is not a file: {}", path.display()));
    }
    fs::remove_file(path)?;
    Ok(())
}

fn workspace_service(context: &ToolContext) -> Result<WorkspaceFileService> {
    Ok(WorkspaceFileService::new(context.workspace.clone()))
}
