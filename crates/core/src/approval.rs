//! Approval policy and user review contract.
//!
//! This module classifies tool calls before execution and produces stable mobile
//! approval requests. It is deliberately conservative: read-only tools can run
//! automatically, while file writes, shell commands, git mutations and unknown
//! tools require explicit user review.

use crate::tool_call::ToolCallRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static APPROVAL_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ApprovalMode {
    #[default]
    ReviewWritesAndCommands,
    Auto,
    AskEveryTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalRisk {
    Benign,
    Destructive,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCategory {
    Safe,
    FileWrite,
    Shell,
    Git,
    Network,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewDecision {
    Approved,
    ApprovedForSession,
    Denied,
    Abort,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MobileApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub category: ToolCategory,
    pub risk: ApprovalRisk,
    pub description: String,
    pub impacts: Vec<String>,
    pub params: Value,
}

impl MobileApprovalRequest {
    pub fn new(
        tool_name: impl Into<String>,
        category: ToolCategory,
        risk: ApprovalRisk,
        params: Value,
    ) -> Self {
        let tool_name = tool_name.into();
        let description = description_for(&tool_name, &category, &params);
        let impacts = impacts_for(&tool_name, &category, &risk, &params);
        Self {
            id: new_approval_id(),
            tool_name,
            category,
            risk,
            description,
            impacts,
            params,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
}

pub fn categorize_tool(tool_name: &str) -> ToolCategory {
    match tool_name {
        "read_file" | "list_dir" | "file_info" | "git_status" | "git_diff" => ToolCategory::Safe,
        "write_file" | "edit_file" | "delete_file" | "file_ops" | "apply_patch" => {
            ToolCategory::FileWrite
        }
        "exec_shell" | "shell" | "run_command" | "terminal" => ToolCategory::Shell,
        "git" | "git_commit" | "git_push" | "git_pull" | "git_checkout" | "git_reset" => {
            ToolCategory::Git
        }
        "http" | "fetch_url" | "download" | "network" => ToolCategory::Network,
        _ => ToolCategory::Unknown,
    }
}

pub fn classify_risk(category: &ToolCategory, params: &Value) -> ApprovalRisk {
    match category {
        ToolCategory::Safe => ApprovalRisk::Benign,
        ToolCategory::FileWrite => {
            if has_destructive_path(params) || has_delete_intent(params) {
                ApprovalRisk::Destructive
            } else {
                ApprovalRisk::Benign
            }
        }
        ToolCategory::Shell => ApprovalRisk::Destructive,
        ToolCategory::Git => {
            let operation = params
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match operation {
                "status" | "diff" | "log" | "show" => ApprovalRisk::Benign,
                _ => ApprovalRisk::Destructive,
            }
        }
        ToolCategory::Network => ApprovalRisk::Benign,
        ToolCategory::Unknown => ApprovalRisk::Destructive,
    }
}

pub fn should_request_approval(mode: &ApprovalMode, request: &MobileApprovalRequest) -> bool {
    match mode {
        ApprovalMode::Auto => false,
        ApprovalMode::AskEveryTime => true,
        ApprovalMode::ReviewWritesAndCommands => !matches!(request.category, ToolCategory::Safe),
    }
}

/// Parse `mcp__{server}__{tool}` qualified names from the MCP proxy registry.
pub fn parse_mcp_qualified_name(tool_name: &str) -> Option<(&str, &str)> {
    let rest = tool_name.strip_prefix("mcp__")?;
    let (server, tool) = rest.split_once("__")?;
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    Some((server, tool))
}

pub fn approval_request_for_call(call: &ToolCallRequest) -> MobileApprovalRequest {
    if let Some((server, tool)) = parse_mcp_qualified_name(&call.name) {
        let mut request = MobileApprovalRequest::new(
            call.name.clone(),
            ToolCategory::Network,
            ApprovalRisk::Destructive,
            call.arguments.clone(),
        );
        request.description =
            format!("Run MCP tool '{tool}' on server '{server}' (external process or network)");
        request.impacts = vec![
            format!("MCP server: {server}"),
            format!("MCP tool: {tool}"),
            "Requires server enabled in MCP panel and user approval".to_string(),
        ];
        return request;
    }
    let category = categorize_tool(&call.name);
    let risk = classify_risk(&category, &call.arguments);
    MobileApprovalRequest::new(call.name.clone(), category, risk, call.arguments.clone())
}

fn description_for(tool_name: &str, category: &ToolCategory, params: &Value) -> String {
    match category {
        ToolCategory::Safe => format!("Run read-only tool '{}'", tool_name),
        ToolCategory::FileWrite => format!(
            "Tool '{}' may modify workspace file {}",
            tool_name,
            quoted_path(params).unwrap_or_else(|| "<unknown>".to_string())
        ),
        ToolCategory::Shell => format!(
            "Tool '{}' may run command {}",
            tool_name,
            params
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>")
        ),
        ToolCategory::Git => format!("Tool '{}' may change git repository state", tool_name),
        ToolCategory::Network => format!("Tool '{}' may access network resources", tool_name),
        ToolCategory::Unknown => format!("Tool '{}' has unknown impact", tool_name),
    }
}

fn impacts_for(
    tool_name: &str,
    category: &ToolCategory,
    risk: &ApprovalRisk,
    params: &Value,
) -> Vec<String> {
    let mut impacts = Vec::new();
    impacts.push(format!("Tool: {}", tool_name));
    impacts.push(format!("Category: {:?}", category));
    impacts.push(format!("Risk: {:?}", risk));

    if let Some(path) = quoted_path(params) {
        impacts.push(format!("Target path: {}", path));
    }
    if let Some(command) = params.get("command").and_then(Value::as_str) {
        impacts.push(format!("Command: {}", command));
    }
    impacts
}

fn quoted_path(params: &Value) -> Option<String> {
    params
        .get("path")
        .or_else(|| params.get("file"))
        .or_else(|| params.get("file_path"))
        .or_else(|| params.get("target_path"))
        .and_then(Value::as_str)
        .map(std::string::ToString::to_string)
}

fn has_destructive_path(params: &Value) -> bool {
    quoted_path(params).is_some_and(|path| {
        path == "/"
            || path.contains("../")
            || path.contains(".git/")
            || path.ends_with("Cargo.lock")
    })
}

fn has_delete_intent(params: &Value) -> bool {
    params
        .get("operation")
        .and_then(Value::as_str)
        .is_some_and(|op| matches!(op, "delete" | "remove" | "rm"))
}

fn new_approval_id() -> String {
    let seq = APPROVAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("approval-{}-{}", current_unix_time(), seq)
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        approval_request_for_call, categorize_tool, should_request_approval, ApprovalMode,
        ToolCategory,
    };
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use serde_json::json;

    #[test]
    fn classifies_core_tool_categories() {
        assert_eq!(categorize_tool("read_file"), ToolCategory::Safe);
        assert_eq!(categorize_tool("write_file"), ToolCategory::FileWrite);
        assert_eq!(categorize_tool("exec_shell"), ToolCategory::Shell);
        assert_eq!(categorize_tool("git"), ToolCategory::Git);
    }

    #[test]
    fn requests_approval_for_file_writes() {
        let call = ToolCallRequest::new(
            "write_file",
            json!({"path":"src/main.rs"}),
            ToolCallSource::Manual,
        );
        let request = approval_request_for_call(&call);
        assert!(should_request_approval(
            &ApprovalMode::ReviewWritesAndCommands,
            &request
        ));
    }

    #[test]
    fn does_not_request_approval_for_safe_reads() {
        let call = ToolCallRequest::new(
            "read_file",
            json!({"path":"README.md"}),
            ToolCallSource::Manual,
        );
        let request = approval_request_for_call(&call);
        assert!(!should_request_approval(
            &ApprovalMode::ReviewWritesAndCommands,
            &request
        ));
    }

    #[test]
    fn mcp_proxy_tools_require_approval_with_server_context() {
        let call = ToolCallRequest::new(
            "mcp__demo__echo",
            json!({"message": "hi"}),
            ToolCallSource::Manual,
        );
        let request = approval_request_for_call(&call);
        assert!(should_request_approval(
            &ApprovalMode::ReviewWritesAndCommands,
            &request
        ));
        assert!(request.description.contains("demo"));
        assert!(request.description.contains("echo"));
    }
}
