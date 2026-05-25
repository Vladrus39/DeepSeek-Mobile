//! Tool system for DeepSeek Mobile.
//!
//! This mirrors the important contracts from DeepSeek-TUI without copying the
//! terminal runtime directly. Tools expose structured JSON input, capabilities,
//! approval requirements and a workspace-aware execution context.

pub mod file_ops;
pub mod git;
pub mod github;
pub mod mcp_proxy;
pub mod patch;
pub mod phone_bridge;
pub mod shell;
pub mod snapshots;
pub mod web;

use crate::config::ExternalAccessMode;
use crate::workspace::Workspace;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCapability {
    ReadOnly,
    WritesFiles,
    ExecutesCode,
    Network,
    Git,
    RequiresApproval,
    Sandboxable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalRequirement {
    Auto,
    Suggest,
    Required,
    Never,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub metadata: Option<Value>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            success: false,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[derive(Clone, Debug)]
pub struct ToolContext {
    pub workspace: Workspace,
    pub external_access: ExternalAccessMode,
    pub trusted_external_paths: Vec<PathBuf>,
    pub auto_approve: bool,
    pub github_token: Option<String>,
    pub mcp_registry_path: Option<PathBuf>,
}

impl ToolContext {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace,
            external_access: ExternalAccessMode::WorkspaceOnly,
            trusted_external_paths: Vec::new(),
            auto_approve: false,
            github_token: None,
            mcp_registry_path: None,
        }
    }

    pub fn with_external_access(mut self, mode: ExternalAccessMode) -> Self {
        self.external_access = mode;
        self
    }

    pub fn with_trusted_external_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.trusted_external_paths = paths;
        self
    }

    pub fn with_auto_approve(mut self, auto_approve: bool) -> Self {
        self.auto_approve = auto_approve;
        self
    }

    pub fn with_github_token(mut self, github_token: Option<String>) -> Self {
        self.github_token = github_token;
        self
    }

    pub fn with_mcp_registry_path(mut self, path: Option<PathBuf>) -> Self {
        self.mcp_registry_path = path;
        self
    }

    pub fn resolve_path(&self, raw_path: &str) -> Result<PathBuf> {
        if let Some(path) = self.workspace.resolve_relative_path(raw_path) {
            return Ok(path);
        }

        let candidate = PathBuf::from(raw_path);
        if self.external_access == ExternalAccessMode::AllowedByUserGrant
            && self
                .trusted_external_paths
                .iter()
                .any(|trusted| candidate.starts_with(trusted))
        {
            return Ok(candidate);
        }

        if self.external_access == ExternalAccessMode::AskEveryTime {
            return Err(anyhow!(
                "external path requires explicit user approval: {}",
                raw_path
            ));
        }

        Err(anyhow!("path is outside workspace boundary: {}", raw_path))
    }
}

pub trait ToolSpec: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn capabilities(&self) -> Vec<ToolCapability>;

    fn approval_requirement(&self) -> ApprovalRequirement {
        let capabilities = self.capabilities();
        if capabilities.contains(&ToolCapability::ExecutesCode) {
            ApprovalRequirement::Required
        } else if capabilities.contains(&ToolCapability::WritesFiles)
            || capabilities.contains(&ToolCapability::Git)
            || capabilities.contains(&ToolCapability::Network)
            || capabilities.contains(&ToolCapability::RequiresApproval)
        {
            ApprovalRequirement::Suggest
        } else {
            ApprovalRequirement::Auto
        }
    }

    fn is_read_only(&self) -> bool {
        let capabilities = self.capabilities();
        capabilities.contains(&ToolCapability::ReadOnly)
            && !capabilities.contains(&ToolCapability::WritesFiles)
            && !capabilities.contains(&ToolCapability::ExecutesCode)
            && !capabilities.contains(&ToolCapability::Network)
            && !capabilities.contains(&ToolCapability::Git)
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolSpec>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn ToolSpec>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn ToolSpec> {
        self.tools.get(name).map(|tool| tool.as_ref())
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = self.tools.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    pub fn execute(&self, name: &str, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow!("tool '{}' is not registered", name))?;
        tool.execute(input, context)
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_mobile_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(file_ops::ReadFileTool));
    registry.register(Box::new(file_ops::ReadManyFilesTool));
    registry.register(Box::new(file_ops::WriteFileTool));
    registry.register(Box::new(file_ops::ListDirTool));
    registry.register(Box::new(file_ops::EditFileTool));
    registry.register(Box::new(file_ops::DeleteFileTool));
    registry.register(Box::new(file_ops::CopyFileTool));
    registry.register(Box::new(file_ops::MoveFileTool));
    registry.register(Box::new(file_ops::FileOpsTool));
    registry.register(Box::new(patch::ApplyPatchTool));
    registry.register(Box::new(shell::ShellTool));
    registry.register(Box::new(phone_bridge::PhoneControlTool));
    registry.register(Box::new(git::GitTool));
    registry.register(Box::new(snapshots::CreateSnapshotTool));
    registry.register(Box::new(snapshots::ListSnapshotsTool));
    registry.register(Box::new(snapshots::RestoreSnapshotTool));
    registry.register(Box::new(github::GitHubRepoTool));
    registry.register(Box::new(github::GitHubPRTool));
    registry.register(Box::new(github::GitHubIssueTool));
    registry.register(Box::new(github::GitHubBrowseTool));
    registry.register(Box::new(github::GitHubPushFileTool));
    registry.register(Box::new(web::WebFetchTool));
    registry.register(Box::new(web::WebSearchTool));
    registry
}

pub fn default_mobile_tool_registry_with_mcp(
    mcp_tools: &[crate::mcp::McpToolDescriptor],
) -> ToolRegistry {
    let mut registry = default_mobile_tool_registry();
    mcp_proxy::extend_registry_with_mcp(&mut registry, mcp_tools);
    registry
}

pub fn required_str<'a>(input: &'a Value, key: &str) -> Result<&'a str> {
    input
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing required string field '{}'", key))
}

pub fn optional_str<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::default_mobile_tool_registry;

    #[test]
    fn default_registry_contains_core_mobile_tools() {
        let registry = default_mobile_tool_registry();
        let names = registry.names();
        assert!(names.contains(&"read_file".to_string()));
        assert!(names.contains(&"read_many_files".to_string()));
        assert!(names.contains(&"write_file".to_string()));
        assert!(names.contains(&"list_dir".to_string()));
        assert!(names.contains(&"edit_file".to_string()));
        assert!(names.contains(&"apply_patch".to_string()));
        assert!(names.contains(&"exec_shell".to_string()));
        assert!(names.contains(&"phone_control".to_string()));
        assert!(names.contains(&"git".to_string()));
        assert!(names.contains(&"snapshot_create".to_string()));
        assert!(names.contains(&"snapshot_list".to_string()));
        assert!(names.contains(&"snapshot_restore".to_string()));
        assert!(names.contains(&"github_repo".to_string()));
        assert!(names.contains(&"delete_file".to_string()));
        assert!(names.contains(&"copy_file".to_string()));
        assert!(names.contains(&"move_file".to_string()));
        assert!(names.contains(&"github_browse".to_string()));
        assert!(names.contains(&"web_fetch".to_string()));
        assert!(names.contains(&"web_search".to_string()));
    }
}
