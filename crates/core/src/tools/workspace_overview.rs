//! Summarize workspace layout for the model (symbol/search lite).

use super::{ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::workspace_files::WorkspaceFileService;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const MAX_LIST_DEPTH: usize = 4;
const MAX_ENTRIES_SCANNED: usize = 2_000;

pub struct WorkspaceOverviewTool;

impl ToolSpec for WorkspaceOverviewTool {
    fn name(&self) -> &str {
        "workspace_overview"
    }

    fn description(&self) -> &str {
        "Summarize the active workspace: top-level entries, file counts by extension, and largest paths. Use before large refactors or when orienting in a repo."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "max_depth": {
                    "type": "integer",
                    "description": "Directory depth to scan (default 4, max 6)."
                }
            }
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let max_depth = input
            .get("max_depth")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(MAX_LIST_DEPTH)
            .clamp(1, 6);
        let service = WorkspaceFileService::new(context.workspace.clone());
        let summary = summarize_workspace(&service, max_depth)?;
        Ok(ToolResult::success(summary).with_metadata(json!({
            "workspace_root": context.workspace.root.display().to_string(),
            "executor": format!("{:?}", context.workspace.executor),
        })))
    }
}

fn summarize_workspace(service: &WorkspaceFileService, max_depth: usize) -> Result<String> {
    let root = service.workspace().root.clone();
    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut extensions: BTreeMap<String, usize> = BTreeMap::new();
    let mut largest: Vec<(u64, String)> = Vec::new();
    let mut scanned = 0usize;

    walk(
        &root,
        &root,
        0,
        max_depth,
        &mut file_count,
        &mut dir_count,
        &mut extensions,
        &mut largest,
        &mut scanned,
    )?;

    largest.sort_by(|a, b| b.0.cmp(&a.0));
    let top_large = largest.into_iter().take(8).collect::<Vec<_>>();

    let mut lines = vec![
        format!("Workspace: {}", service.workspace().name),
        format!("Root: {}", root.display()),
        format!("Files: {file_count}, directories: {dir_count} (scanned up to depth {max_depth})"),
    ];
    if scanned >= MAX_ENTRIES_SCANNED {
        lines.push(format!(
            "Note: scan capped at {MAX_ENTRIES_SCANNED} entries — use list_dir/read_file for more."
        ));
    }
    if !extensions.is_empty() {
        lines.push("Extensions:".to_string());
        for (ext, count) in extensions.iter().take(12) {
            let label = if ext.is_empty() {
                "(no ext)"
            } else {
                ext.as_str()
            };
            lines.push(format!("  {label}: {count}"));
        }
    }
    if !top_large.is_empty() {
        lines.push("Largest files:".to_string());
        for (size, path) in top_large {
            lines.push(format!("  {path} ({size} bytes)"));
        }
    }

    Ok(lines.join("\n"))
}

fn walk(
    root: &std::path::Path,
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    file_count: &mut usize,
    dir_count: &mut usize,
    extensions: &mut BTreeMap<String, usize>,
    largest: &mut Vec<(u64, String)>,
    scanned: &mut usize,
) -> Result<()> {
    if depth > max_depth || *scanned >= MAX_ENTRIES_SCANNED {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        if *scanned >= MAX_ENTRIES_SCANNED {
            break;
        }
        *scanned += 1;
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".deepseek-mobile" || name == "target" || name == "node_modules" {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            *dir_count += 1;
            walk(
                root,
                &path,
                depth + 1,
                max_depth,
                file_count,
                dir_count,
                extensions,
                largest,
                scanned,
            )?;
        } else if metadata.is_file() {
            *file_count += 1;
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
            *extensions.entry(ext).or_insert(0) += 1;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            largest.push((metadata.len(), rel));
        }
    }
    Ok(())
}
