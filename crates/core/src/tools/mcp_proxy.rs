//! Proxy tools that forward execution to connected MCP servers.

use super::{ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::mcp::McpToolDescriptor;
use crate::mcp_client::{default_mcp_path, invoke_mcp_tool_at_path};
use anyhow::Result;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct McpProxyTool {
    qualified_name: String,
    server: String,
    tool_name: String,
    description: String,
    input_schema: Value,
}

impl McpProxyTool {
    pub fn from_descriptor(descriptor: &McpToolDescriptor) -> Self {
        let tool_name = descriptor.name.clone();
        let qualified_name = format!("mcp__{}__{}", descriptor.server, tool_name);
        Self {
            qualified_name,
            server: descriptor.server.clone(),
            tool_name,
            description: descriptor
                .description
                .clone()
                .unwrap_or_else(|| format!("MCP tool from server '{}'", descriptor.server)),
            input_schema: descriptor.input_schema.clone(),
        }
    }

    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }
}

impl ToolSpec for McpProxyTool {
    fn name(&self) -> &str {
        &self.qualified_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::Network,
            ToolCapability::RequiresApproval,
        ]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let registry_path = context
            .mcp_registry_path
            .clone()
            .unwrap_or_else(default_mcp_path);
        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(invoke_mcp_tool_at_path(
                &registry_path,
                &self.server,
                &self.tool_name,
                input,
            ))
        } else {
            futures::executor::block_on(invoke_mcp_tool_at_path(
                &registry_path,
                &self.server,
                &self.tool_name,
                input,
            ))
        };
        match result {
            Ok(content) => Ok(ToolResult::success(content)),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

pub fn extend_registry_with_mcp(
    registry: &mut super::ToolRegistry,
    descriptors: &[McpToolDescriptor],
) {
    for descriptor in descriptors {
        if descriptor.server.is_empty() {
            continue;
        }
        let proxy = McpProxyTool::from_descriptor(descriptor);
        registry.register(Box::new(proxy));
    }
}
