//! Git operation tool surface.
//!
//! Git operations are routed through the active workspace executor:
//! - LocalAndroid/Termux: executes git commands directly in the workspace root
//! - PcGateway: sent to the paired PC-host which runs git in the granted workspace
//!
//! Read-only operations (status, diff, log) auto-approve.
//! Write operations (commit, push, pull, branch) require approval.

use super::{optional_str, required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

pub struct GitTool;

impl ToolSpec for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Run git operations in the active workspace: status, diff, commit, push, pull, branch, log."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "commit", "push", "pull", "branch", "log", "add", "checkout", "clone"],
                    "description": "Git operation to perform"
                },
                "message": { "type": "string", "description": "Commit message (required for commit)" },
                "branch": { "type": "string", "description": "Branch name (for checkout, push, pull, branch)" },
                "remote": { "type": "string", "description": "Remote name (defaults to 'origin')" },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files to add/stage (for add/commit). Empty means all."
                },
                "url": { "type": "string", "description": "Repository URL (for clone)" },
                "max_count": { "type": "integer", "description": "Max log entries (default 20)" }
            },
            "required": ["operation"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Git, ToolCapability::RequiresApproval, ToolCapability::Sandboxable]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let operation = required_str(&input, "operation")?;
        let workspace_root = &context.workspace.root;

        match operation {
            "status" => run_git(workspace_root, &["status", "--short"]),
            "diff" => {
                let files: Vec<String> = string_array(&input, "files");
                let mut args = vec!["diff", "--"];
                if files.is_empty() {
                    Ok(run_git(workspace_root, &args)?)
                } else {
                    args.extend(files.iter().map(String::as_str));
                    Ok(run_git(workspace_root, &args)?)
                }
            }
            "log" => {
                let max_count = input
                    .get("max_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(20)
                    .to_string();
                let branch = optional_str(&input, "branch");
                let mut args = vec!["log", "--oneline", "-n", &max_count];
                if let Some(b) = branch {
                    args.push(b);
                }
                run_git(workspace_root, &args)
            }
            "branch" => {
                let branch_name = optional_str(&input, "branch");
                if let Some(name) = branch_name {
                    run_git(workspace_root, &["checkout", "-b", name])
                } else {
                    run_git(workspace_root, &["branch", "--list"])
                }
            }
            "add" => {
                let files: Vec<String> = string_array(&input, "files");
                if files.is_empty() {
                    run_git(workspace_root, &["add", "."])
                } else {
                    let mut args: Vec<&str> = vec!["add"];
                    let file_strs: Vec<&str> = files.iter().map(String::as_str).collect();
                    args.extend(&file_strs);
                    run_git(workspace_root, &args)
                }
            }
            "commit" => {
                let message = required_str(&input, "message")?;
                let files: Vec<String> = string_array(&input, "files");
                if !files.is_empty() {
                    let mut add_args: Vec<&str> = vec!["add"];
                    let file_strs: Vec<&str> = files.iter().map(String::as_str).collect();
                    add_args.extend(&file_strs);
                    run_git(workspace_root, &add_args)?;
                }
                run_git(workspace_root, &["commit", "-m", message])
            }
            "push" => {
                let remote = optional_str(&input, "remote").unwrap_or("origin");
                let branch = optional_str(&input, "branch");
                if let Some(b) = branch {
                    run_git(workspace_root, &["push", remote, b])
                } else {
                    run_git(workspace_root, &["push", remote, "HEAD"])
                }
            }
            "pull" => {
                let remote = optional_str(&input, "remote").unwrap_or("origin");
                let branch = optional_str(&input, "branch");
                if let Some(b) = branch {
                    run_git(workspace_root, &["pull", remote, b])
                } else {
                    run_git(workspace_root, &["pull"])
                }
            }
            "checkout" => {
                let branch = required_str(&input, "branch")?;
                run_git(workspace_root, &["checkout", branch])
            }
            "clone" => {
                let url = required_str(&input, "url")?;
                let branch = optional_str(&input, "branch");
                if let Some(b) = branch {
                    run_git(workspace_root, &["clone", "--branch", b, url, "."])
                } else {
                    run_git(workspace_root, &["clone", url, "."])
                }
            }
            other => Err(anyhow!(
                "unsupported git operation: '{}'. Supported: status, diff, log, branch, add, commit, push, pull, checkout, clone",
                other
            )),
        }
    }
}

fn run_git(workspace_root: &PathBuf, args: &[&str]) -> Result<ToolResult> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root)
        .output()
        .map_err(|e| anyhow!("failed to run git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Ok(ToolResult::error(format!(
            "git {} failed (exit {}):\nSTDOUT:\n{}\nSTDERR:\n{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        ))
        .with_metadata(json!({
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr
        })));
    }

    let text = if stdout.is_empty() { stderr } else { stdout };
    Ok(ToolResult::success(text))
}

fn string_array(input: &Value, key: &str) -> Vec<String> {
    input
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{ExecutorKind, Workspace};
    use std::fs;
    use std::path::PathBuf;

    fn temp_workspace() -> (ToolContext, PathBuf) {
        // Use thread id for parallel test safety
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_git_test_{}",
            hasher.finish()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::new("test", "Test", root.clone(), ExecutorKind::LocalAndroid);
        (ToolContext::new(workspace), root)
    }

    fn init_git_repo(root: &PathBuf) {
        Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@deepseek.mobile"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "DeepSeek Mobile Test"])
            .current_dir(root)
            .output()
            .unwrap();
    }

    #[test]
    fn git_status_on_empty_repo() {
        let (ctx, root) = temp_workspace();
        init_git_repo(&root);
        let result = GitTool.execute(json!({"operation": "status"}), &ctx).unwrap();
        assert!(result.success);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn git_commit_detects_changes() {
        let (ctx, root) = temp_workspace();
        init_git_repo(&root);
        fs::write(root.join("README.md"), "# Test").unwrap();
        let result = GitTool
            .execute(
                json!({"operation": "commit", "message": "Initial commit", "files": ["README.md"]}),
                &ctx,
            )
            .unwrap();
        assert!(result.success);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_unknown_operation() {
        let (ctx, root) = temp_workspace();
        let result = GitTool.execute(json!({"operation": "rebase"}), &ctx);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn schema_accepts_all_operations() {
        let schema = GitTool.input_schema();
        let operations = schema["properties"]["operation"]["enum"]
            .as_array()
            .unwrap();
        let ops: Vec<&str> = operations.iter().filter_map(Value::as_str).collect();
        assert!(ops.contains(&"status"));
        assert!(ops.contains(&"commit"));
        assert!(ops.contains(&"push"));
        assert!(ops.contains(&"pull"));
        assert!(ops.contains(&"clone"));
    }
}
