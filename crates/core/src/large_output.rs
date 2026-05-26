//! Route oversized tool output to workspace files (TUI large-output pattern).

use crate::tools::ToolResult;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_MAX_TOOL_RESULT_CHARS: usize = 12_000;
pub const DEFAULT_MAX_TOOL_RESULT_LINES: usize = 400;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutedToolOutput {
    pub model_text: String,
    pub spilled: bool,
    pub spill_path: Option<String>,
    pub original_chars: usize,
}

pub fn route_tool_result_for_model(
    tool_name: &str,
    result: &ToolResult,
    workspace: &Workspace,
    max_chars: usize,
) -> Result<RoutedToolOutput> {
    let body = if result.success {
        result.content.clone()
    } else {
        format!("ERROR: {}", result.content)
    };
    let original_chars = body.chars().count();
    if original_chars <= max_chars && body.lines().count() <= DEFAULT_MAX_TOOL_RESULT_LINES {
        return Ok(RoutedToolOutput {
            model_text: body,
            spilled: false,
            spill_path: None,
            original_chars,
        });
    }

    let spill_relative = spill_relative_path(tool_name);
    let spill_absolute = workspace
        .resolve_relative_path(&spill_relative)
        .unwrap_or_else(|| workspace.root.join(&spill_relative));
    if let Some(parent) = spill_absolute.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create tool output dir {}", parent.display()))?;
    }
    fs::write(&spill_absolute, &body)
        .with_context(|| format!("write spilled tool output {}", spill_absolute.display()))?;

    let preview = truncate_for_model(&body, max_chars);
    let model_text = format!(
        "Tool `{}` produced {} characters ({} lines). Full output saved to workspace file `{}`.\n\nPreview:\n{}",
        tool_name,
        original_chars,
        body.lines().count(),
        spill_relative,
        preview
    );

    Ok(RoutedToolOutput {
        model_text,
        spilled: true,
        spill_path: Some(spill_relative),
        original_chars,
    })
}

pub fn format_tool_results_message(tool_name: &str, routed: &RoutedToolOutput) -> String {
    if routed.spilled {
        format!(
            "[Tool result: {tool_name}]\n{}\n\n(Read full output with read_file at `{}`.)",
            routed.model_text,
            routed.spill_path.as_deref().unwrap_or("unknown")
        )
    } else {
        format!("[Tool result: {tool_name}]\n{}", routed.model_text)
    }
}

fn spill_relative_path(tool_name: &str) -> String {
    let safe: String = tool_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(".deepseek-mobile/tool-output/{}-{}.txt", safe, nanos)
}

fn truncate_for_model(body: &str, max_chars: usize) -> String {
    let line_limited: String = body
        .lines()
        .take(DEFAULT_MAX_TOOL_RESULT_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    if line_limited.chars().count() <= max_chars {
        return line_limited;
    }
    line_limited.chars().take(max_chars).collect::<String>() + "\n... (truncated)"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{ExecutorKind, Workspace};
    use std::path::PathBuf;

    #[test]
    fn small_output_not_spilled() {
        let workspace = Workspace::new("w1", "W", PathBuf::from("."), ExecutorKind::LocalAndroid);
        let result = ToolResult::success("hello");
        let routed = route_tool_result_for_model("read_file", &result, &workspace, 1000).unwrap();
        assert!(!routed.spilled);
        assert_eq!(routed.model_text, "hello");
    }

    #[test]
    fn large_output_spills_to_workspace() {
        let dir = std::env::temp_dir().join(format!("deepseek-spill-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workspace = Workspace::new("w1", "W", dir.clone(), ExecutorKind::LocalAndroid);
        let big = "x".repeat(20_000);
        let result = ToolResult::success(big);
        let routed = route_tool_result_for_model("exec_shell", &result, &workspace, 500).unwrap();
        assert!(routed.spilled);
        assert!(routed.spill_path.is_some());
        let spill = dir.join(routed.spill_path.as_ref().unwrap());
        assert!(spill.exists());
        assert!(routed.model_text.contains("Preview:"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
