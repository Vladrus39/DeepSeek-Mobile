//! Read short summaries of common project entry files.

use super::{ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::workspace_files::WorkspaceFileService;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

const MAX_FILE_CHARS: usize = 8_000;
const CANDIDATE_FILES: &[&str] = &[
    "README.md",
    "README",
    "readme.md",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "Makefile",
    "docs/MASTER_PLAN.md",
    "ROADMAP.md",
];

pub struct FileSummaryTool;

impl ToolSpec for FileSummaryTool {
    fn name(&self) -> &str {
        "file_summary"
    }

    fn description(&self) -> &str {
        "Summarize key project files (README, manifests, roadmap) for quick orientation. Optional path list; defaults to common entry files at workspace root."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Relative workspace paths to summarize (max 8)."
                }
            }
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let service = WorkspaceFileService::new(context.workspace.clone());
        let paths: Vec<String> = input
            .get("paths")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .take(8)
                    .collect()
            })
            .unwrap_or_default();

        let targets = if paths.is_empty() {
            CANDIDATE_FILES
                .iter()
                .map(|path| (*path).to_string())
                .collect()
        } else {
            paths
        };

        let mut sections = Vec::new();
        for relative in targets {
            if let Some(section) = summarize_path(&service, Path::new(&relative))? {
                sections.push(section);
            }
        }

        if sections.is_empty() {
            return Ok(ToolResult::success(
                "No summary files found. Try workspace_overview or list_dir first.",
            ));
        }

        Ok(ToolResult::success(sections.join("\n\n---\n\n")))
    }
}

fn summarize_path(service: &WorkspaceFileService, relative: &Path) -> Result<Option<String>> {
    let absolute = service.workspace().resolve_relative_path(relative);
    let Some(absolute) = absolute else {
        return Ok(None);
    };
    if !absolute.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&absolute)?;
    let line_count = content.lines().count();
    let preview = if content.chars().count() > MAX_FILE_CHARS {
        content.chars().take(MAX_FILE_CHARS).collect::<String>() + "\n... (truncated)"
    } else {
        content
    };
    Ok(Some(format!(
        "## {} ({} lines)\n{}",
        relative.display(),
        line_count,
        preview
    )))
}
