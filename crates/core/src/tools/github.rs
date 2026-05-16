//! GitHub tool surface for DeepSeek Mobile.
//!
//! These tools let the model interact with GitHub repositories: browse files,
//! create pull requests, manage issues, and push changes directly through the
//! GitHub REST API. Every operation that modifies repository state requires
//! explicit approval from the mobile UI.

use super::{optional_str, required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::github::{GitHubClient, GitHubRepo};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub struct GitHubRepoTool;
pub struct GitHubPRTool;
pub struct GitHubIssueTool;
pub struct GitHubBrowseTool;
pub struct GitHubPushFileTool;

impl ToolSpec for GitHubRepoTool {
    fn name(&self) -> &str {
        "github_repo"
    }

    fn description(&self) -> &str {
        "Get repository information, list branches, or browse file contents from GitHub."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository in owner/repo format" },
                "action": { "type": "string", "enum": ["info", "branches", "read", "list"], "description": "Action: info, branches, read file, list directory" },
                "path": { "type": "string", "description": "File or directory path for read/list actions" },
                "branch": { "type": "string", "description": "Branch name for read/list actions" }
            },
            "required": ["repo", "action"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Network]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let github = github_client_from_context(context)?;
        let repo = GitHubRepo::parse(required_str(&input, "repo")?)?;
        let action = required_str(&input, "action")?;

        let rt = tokio::runtime::Handle::current();
        match action {
            "info" => {
                let info = rt.block_on(github.get_repo(&repo))?;
                Ok(ToolResult::success(serde_json::to_string_pretty(&info)?)
                    .with_metadata(serde_json::to_value(info)?))
            }
            "branches" => {
                let branches = rt.block_on(github.list_branches(&repo))?;
                Ok(ToolResult::success(serde_json::to_string_pretty(&branches)?)
                    .with_metadata(serde_json::to_value(branches)?))
            }
            "read" => {
                let path = required_str(&input, "path")?;
                let branch = optional_str(&input, "branch");
                let file = rt.block_on(github.get_file_content(&repo, path, branch))?;
                Ok(ToolResult::success(file.content)
                    .with_metadata(json!({"path": file.path, "sha": file.sha})))
            }
            "list" => {
                let path = optional_str(&input, "path").unwrap_or("");
                let branch = optional_str(&input, "branch");
                let entries = rt.block_on(github.list_contents(&repo, path, branch))?;
                Ok(ToolResult::success(serde_json::to_string_pretty(&entries)?)
                    .with_metadata(serde_json::to_value(entries)?))
            }
            other => Err(anyhow!("unsupported github_repo action: {}", other)),
        }
    }
}

impl ToolSpec for GitHubPRTool {
    fn name(&self) -> &str {
        "github_pr"
    }

    fn description(&self) -> &str {
        "Create or list pull requests on GitHub. Creating a PR requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository in owner/repo format" },
                "action": { "type": "string", "enum": ["create", "list"], "description": "Action: create or list PRs" },
                "title": { "type": "string", "description": "PR title (required for create)" },
                "body": { "type": "string", "description": "PR description" },
                "head": { "type": "string", "description": "Head branch (required for create)" },
                "base": { "type": "string", "description": "Base branch (required for create)" },
                "state": { "type": "string", "enum": ["open", "closed", "all"], "description": "Filter PRs by state (for list)" }
            },
            "required": ["repo", "action"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Network, ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let github = github_client_from_context(context)?;
        let repo = GitHubRepo::parse(required_str(&input, "repo")?)?;
        let action = required_str(&input, "action")?;

        let rt = tokio::runtime::Handle::current();
        match action {
            "create" => {
                let title = required_str(&input, "title")?;
                let body = optional_str(&input, "body").unwrap_or("");
                let head = required_str(&input, "head")?;
                let base = required_str(&input, "base")?;
                let pr = rt.block_on(github.create_pr(&repo, title, body, head, base))?;
                Ok(ToolResult::success(format!(
                    "Created PR #{}: {} ({})",
                    pr.number, pr.title, pr.html_url
                ))
                .with_metadata(serde_json::to_value(pr)?))
            }
            "list" => {
                let state = optional_str(&input, "state");
                let prs = rt.block_on(github.list_prs(&repo, state))?;
                Ok(ToolResult::success(serde_json::to_string_pretty(&prs)?)
                    .with_metadata(serde_json::to_value(prs)?))
            }
            other => Err(anyhow!("unsupported github_pr action: {}", other)),
        }
    }
}

impl ToolSpec for GitHubIssueTool {
    fn name(&self) -> &str {
        "github_issue"
    }

    fn description(&self) -> &str {
        "Create or list issues on GitHub."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository in owner/repo format" },
                "action": { "type": "string", "enum": ["create", "list"], "description": "Action: create or list issues" },
                "title": { "type": "string", "description": "Issue title (required for create)" },
                "body": { "type": "string", "description": "Issue description" },
                "labels": { "type": "array", "items": {"type": "string"}, "description": "Labels for the issue" },
                "state": { "type": "string", "enum": ["open", "closed", "all"], "description": "Filter issues by state (for list)" }
            },
            "required": ["repo", "action"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Network, ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let github = github_client_from_context(context)?;
        let repo = GitHubRepo::parse(required_str(&input, "repo")?)?;
        let action = required_str(&input, "action")?;

        let rt = tokio::runtime::Handle::current();
        match action {
            "create" => {
                let title = required_str(&input, "title")?;
                let body = optional_str(&input, "body").unwrap_or("");
                let labels: Vec<String> = input
                    .get("labels")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let issue = rt.block_on(github.create_issue(&repo, title, body, &labels))?;
                Ok(ToolResult::success(format!(
                    "Created issue #{}: {} ({})",
                    issue.number, issue.title, issue.html_url
                ))
                .with_metadata(serde_json::to_value(issue)?))
            }
            "list" => {
                let state = optional_str(&input, "state");
                let issues = rt.block_on(github.list_issues(&repo, state))?;
                Ok(ToolResult::success(serde_json::to_string_pretty(&issues)?)
                    .with_metadata(serde_json::to_value(issues)?))
            }
            other => Err(anyhow!("unsupported github_issue action: {}", other)),
        }
    }
}

impl ToolSpec for GitHubBrowseTool {
    fn name(&self) -> &str {
        "github_browse"
    }

    fn description(&self) -> &str {
        "Browse GitHub repository file tree and contents."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository in owner/repo format" },
                "path": { "type": "string", "description": "Directory or file path" },
                "branch": { "type": "string", "description": "Branch name (defaults to default branch)" }
            },
            "required": ["repo"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::Network]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let github = github_client_from_context(context)?;
        let repo = GitHubRepo::parse(required_str(&input, "repo")?)?;
        let path = optional_str(&input, "path").unwrap_or("");
        let branch = optional_str(&input, "branch");

        let rt = tokio::runtime::Handle::current();
        let entries = rt.block_on(github.list_contents(&repo, path, branch))?;

        let mut output = String::new();
        for entry in &entries {
            let prefix = if entry.entry_type == "dir" { "📁" } else { "📄" };
            output.push_str(&format!(
                "{} {} ({})\n",
                prefix, entry.name, entry.sha.chars().take(7).collect::<String>()
            ));
        }

        Ok(ToolResult::success(output)
            .with_metadata(serde_json::to_value(entries)?))
    }
}

impl ToolSpec for GitHubPushFileTool {
    fn name(&self) -> &str {
        "github_push_file"
    }

    fn description(&self) -> &str {
        "Push a single file change to GitHub by committing through the REST API. Requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository in owner/repo format" },
                "path": { "type": "string", "description": "File path in the repository" },
                "content": { "type": "string", "description": "New file content" },
                "message": { "type": "string", "description": "Commit message" },
                "branch": { "type": "string", "description": "Branch to commit to" },
                "sha": { "type": "string", "description": "SHA of the file being replaced (required for updates)" }
            },
            "required": ["repo", "path", "content", "message", "branch"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::WritesFiles, ToolCapability::Network, ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let github = github_client_from_context(context)?;
        let repo = GitHubRepo::parse(required_str(&input, "repo")?)?;
        let path = required_str(&input, "path")?;
        let content = required_str(&input, "content")?;
        let message = required_str(&input, "message")?;
        let branch = required_str(&input, "branch")?;
        let sha = optional_str(&input, "sha");

        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(github.create_or_update_file(
            &repo, path, content, message, branch, sha,
        ))?;

        Ok(ToolResult::success(format!(
            "Pushed {} to {}: {}",
            path, repo.full_name(), result.html_url
        ))
        .with_metadata(json!({"sha": result.sha, "html_url": result.html_url})))
    }
}

fn github_client_from_context(_context: &ToolContext) -> Result<GitHubClient> {
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("DEEPSEEK_GITHUB_TOKEN"))
        .unwrap_or_default();
    if token.is_empty() {
        return Err(anyhow!(
            "GitHub token is not configured. Set GITHUB_TOKEN or DEEPSEEK_GITHUB_TOKEN environment variable."
        ));
    }
    Ok(GitHubClient::new(token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{ExecutorKind, Workspace};
    use std::path::PathBuf;

    #[allow(dead_code)]
    fn tool_context() -> ToolContext {
        let workspace = Workspace::new(
            "test-workspace",
            "Test Workspace",
            PathBuf::from("."),
            ExecutorKind::LocalAndroid,
        );
        ToolContext::new(workspace)
    }

    #[test]
    fn github_repo_tool_registers() {
        let tool = GitHubRepoTool;
        assert_eq!(tool.name(), "github_repo");
        assert!(!tool.input_schema().is_null());
    }

    #[test]
    fn github_pr_tool_registers() {
        let tool = GitHubPRTool;
        assert_eq!(tool.name(), "github_pr");
    }

    #[test]
    fn github_browse_tool_registers() {
        let tool = GitHubBrowseTool;
        assert_eq!(tool.name(), "github_browse");
        assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
    }
}
