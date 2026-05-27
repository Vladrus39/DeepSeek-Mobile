//! Shell execution tool surface.
//!
//! Direct shell execution is intentionally disabled in the mobile core until a
//! concrete executor is selected. Android builds should route commands through
//! Termux or a remote backend instead of spawning arbitrary local processes.

use super::{required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use anyhow::Result;
use serde_json::{json, Value};

pub struct ShellTool;

impl ToolSpec for ShellTool {
    fn name(&self) -> &str {
        "exec_shell"
    }

    fn description(&self) -> &str {
        "Request shell command execution through the selected executor. Requires approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "timeout_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional command timeout in seconds for executors that support it"
                }
            },
            "required": ["command"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::ExecutesCode,
            ToolCapability::RequiresApproval,
            ToolCapability::Sandboxable,
        ]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult> {
        let command = required_str(&input, "command")?;
        Ok(ToolResult::success(format!(
            "Shell execution requested but no executor is active yet. Requested command: {}",
            command
        )))
    }
}
