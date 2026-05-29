//! Run snapshot restore outside the agent tool loop (Snapshots panel confirm).

use crate::mobile_runtime_config::MobileRuntimeConfig;
use anyhow::{anyhow, Result};
use deepseek_mobile_core::{
    tools::{default_mobile_tool_registry, ToolContext, ToolSpec},
    workspace_connection::WorkspaceBackendKind,
    Config, PcGatewayClient, Workspace,
};
use serde_json::json;

pub async fn restore_snapshot_by_id(
    config: Config,
    runtime: MobileRuntimeConfig,
    snapshot_id: &str,
) -> Result<String> {
    let registry = default_mobile_tool_registry();
    let tool = registry
        .get("snapshot_restore")
        .ok_or_else(|| anyhow!("snapshot_restore tool missing"))?;

    if let Some(connection) = runtime.workspace_connection.clone() {
        if connection.backend == WorkspaceBackendKind::PcGateway {
            let Some(gateway_config) = connection.pc_gateway.clone() else {
                return Err(anyhow!("PC workspace active but gateway config is missing"));
            };
            let client = PcGatewayClient::new(gateway_config);
            let response = client
                .restore_snapshot(&connection.workspace_id, snapshot_id)
                .await?;
            return Ok(format!("{:?}", response));
        }
    }

    let workspace = Workspace::new(
        "mobile",
        "Mobile workspace",
        runtime.workspace_root.clone(),
        deepseek_mobile_core::workspace::ExecutorKind::LocalAndroid,
    );
    let context = ToolContext::new(workspace);
    let result = tool.execute(
        json!({ "snapshot_id": snapshot_id }),
        &context,
    )?;
    if !result.success {
        return Err(anyhow!("{}", result.content));
    }
    Ok(result.content)
}
