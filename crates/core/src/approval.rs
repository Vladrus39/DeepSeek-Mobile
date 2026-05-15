//! Approval and risk policy for mobile tool execution.
//!
//! The original TUI separates benign read-only operations from destructive
//! actions. The mobile app needs the same idea, but rendered as approval cards
//! instead of terminal modals.

use crate::tools::{ApprovalRequirement, ToolCapability, ToolSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalMode {
    Auto,
    Suggest,
    Never,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewDecision {
    Approved,
    ApprovedForSession,
    Denied,
    Abort,
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
pub enum ApprovalRisk {
    Benign,
    Destructive,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MobileApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub description: String,
    pub category: ToolCategory,
    pub risk: ApprovalRisk,
    pub requirement: ApprovalRequirement,
    pub params: Value,
    pub impacts: Vec<String>,
}

impl MobileApprovalRequest {
    pub fn new(
        id: impl Into<String>,
        tool: &dyn ToolSpec,
        params: Value,
    ) -> Self {
        let tool_name = tool.name().to_string();
        let category = categorize_tool(&tool_name, &tool.capabilities());
        let risk = classify_risk(&category);
        let requirement = tool.approval_requirement();
        let impacts = build_impact_summary(&tool_name, &category, &params);

        Self {
            id: id.into(),
            tool_name,
            description: tool.description().to_string(),
            category,
            risk,
            requirement,
            params,
            impacts,
        }
    }
}

pub fn categorize_tool(name: &str, capabilities: &[ToolCapability]) -> ToolCategory {
    if capabilities.contains(&ToolCapability::ExecutesCode) || name == "exec_shell" {
        ToolCategory::Shell
    } else if capabilities.contains(&ToolCapability::WritesFiles)
        || matches!(name, "write_file" | "edit_file" | "apply_patch")
    {
        ToolCategory::FileWrite
    } else if capabilities.contains(&ToolCapability::Git) || name.starts_with("git") {
        ToolCategory::Git
    } else if capabilities.contains(&ToolCapability::Network) {
        ToolCategory::Network
    } else if capabilities.contains(&ToolCapability::ReadOnly) {
        ToolCategory::Safe
    } else {
        ToolCategory::Unknown
    }
}

pub fn classify_risk(category: &ToolCategory) -> ApprovalRisk {
    match category {
        ToolCategory::Safe => ApprovalRisk::Benign,
        ToolCategory::FileWrite
        | ToolCategory::Shell
        | ToolCategory::Git
        | ToolCategory::Network
        | ToolCategory::Unknown => ApprovalRisk::Destructive,
    }
}

pub fn should_request_approval(
    mode: &ApprovalMode,
    requirement: &ApprovalRequirement,
    risk: &ApprovalRisk,
) -> bool {
    match mode {
        ApprovalMode::Never => false,
        ApprovalMode::Auto => matches!(requirement, ApprovalRequirement::Required)
            && matches!(risk, ApprovalRisk::Destructive),
        ApprovalMode::Suggest => !matches!(requirement, ApprovalRequirement::Auto),
    }
}

fn build_impact_summary(tool_name: &str, category: &ToolCategory, params: &Value) -> Vec<String> {
    let mut impacts = Vec::new();
    match category {
        ToolCategory::Safe => impacts.push("Read-only operation".to_string()),
        ToolCategory::FileWrite => impacts.push("May modify files in the workspace".to_string()),
        ToolCategory::Shell => impacts.push("May execute commands through an executor".to_string()),
        ToolCategory::Git => impacts.push("May inspect or change repository state".to_string()),
        ToolCategory::Network => impacts.push("May access network resources".to_string()),
        ToolCategory::Unknown => impacts.push("Unknown tool impact".to_string()),
    }

    if let Some(path) = params.get("path").and_then(Value::as_str) {
        impacts.push(format!("Path: {}", path));
    }
    if let Some(command) = params.get("command").and_then(Value::as_str) {
        impacts.push(format!("Command: {}", command));
    }
    if impacts.is_empty() {
        impacts.push(format!("Tool: {}", tool_name));
    }
    impacts
}

#[cfg(test)]
mod tests {
    use super::{classify_risk, should_request_approval, ApprovalMode, ApprovalRisk, ToolCategory};
    use crate::tools::ApprovalRequirement;

    #[test]
    fn safe_tools_are_benign() {
        assert_eq!(classify_risk(&ToolCategory::Safe), ApprovalRisk::Benign);
    }

    #[test]
    fn shell_tools_are_destructive() {
        assert_eq!(classify_risk(&ToolCategory::Shell), ApprovalRisk::Destructive);
    }

    #[test]
    fn suggest_mode_requests_suggested_approval() {
        assert!(should_request_approval(
            &ApprovalMode::Suggest,
            &ApprovalRequirement::Suggest,
            &ApprovalRisk::Destructive,
        ));
    }

    #[test]
    fn auto_mode_still_requests_required_destructive_approval() {
        assert!(should_request_approval(
            &ApprovalMode::Auto,
            &ApprovalRequirement::Required,
            &ApprovalRisk::Destructive,
        ));
    }
}
