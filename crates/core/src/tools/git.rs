//! Git operation tool surface.
//!
//! Real git execution will be routed through Termux or a remote executor. The
//! core exposes a structured tool contract now so the agent can reason about
//! git operations safely before execution is enabled.

use super::{ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec, required_str};
use anyhow::Result;
use serde_json::{Value, json};

pub struct GitTool;

impl ToolSpec for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Request a git operation such as status, diff, commit, pull or push."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string", "description": "Git operation: status, diff, commit, pull, push" },
                "args": { "type": "string", "description": "Optional raw git arguments" }
            },
            "required": ["operation"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Git, ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult> {
        let operation = required_str(&input, "operation")?;
        let args = input.get("args").and_then(Value::as_str).unwrap_or("");
        Ok(ToolResult::success(format!(
            "Git operation requested but no git executor is active yet. Operation: {} {}",
            operation, args
        )))
    }
}
