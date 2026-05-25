//! Open a path in the OS file manager (PC Host workspace or trusted grant).

use super::{required_str, ToolCapability, ToolContext, ToolResult, ToolSpec};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub struct OpenPathTool;

impl ToolSpec for OpenPathTool {
    fn name(&self) -> &str {
        "open_path"
    }

    fn description(&self) -> &str {
        "Open a file or folder in the system file manager. On a paired PC workspace this runs on the PC (Explorer/Finder/xdg-open). Relative paths are inside the project; absolute paths require trusted-path grant mode on phone and matching DEEPSEEK_PC_HOST_TRUSTED_PATHS on the PC host."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative workspace path or absolute path covered by trusted grants."
                }
            },
            "required": ["path"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::RequiresApproval]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = required_str(&input, "path")?;
        let _resolved = context.resolve_path(path)?;
        Err(anyhow!(
            "open_path is routed by ToolExecutionCoordinator: activate a PC Host workspace or use phone_control on device"
        ))
    }
}
