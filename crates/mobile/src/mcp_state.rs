use deepseek_mobile_core::{
    connect_mcp_server, tools_for_server, McpClientRegistry, McpServerStatus,
};
use std::path::PathBuf;

use crate::mobile_runtime_config::default_data_dir;

#[derive(Clone, Debug)]
pub struct McpUiState {
    pub registry: McpClientRegistry,
    pub last_error: Option<String>,
    pub mcp_path: PathBuf,
}

impl Default for McpUiState {
    fn default() -> Self {
        let mcp_path = default_data_dir().join("mcp.json");
        Self {
            registry: McpClientRegistry::default(),
            last_error: None,
            mcp_path,
        }
    }
}

impl McpUiState {
    /// Load MCP servers from disk.
    pub fn refresh(&mut self) {
        match McpClientRegistry::load_or_default(&self.mcp_path) {
            Ok(reg) => {
                self.registry = reg;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to load MCP servers: {}", e));
            }
        }
    }

    /// Save current registry to disk.
    pub fn save(&mut self) {
        if let Err(e) = self.registry.save(&self.mcp_path) {
            self.last_error = Some(format!("Failed to save MCP config: {}", e));
        }
    }

    /// Toggle server enabled/disabled.
    pub fn toggle_server(&mut self, name: &str, enabled: bool) {
        self.registry.set_enabled(name, enabled);
        self.save();
    }

    /// Remove a server.
    pub fn remove_server(&mut self, name: &str) {
        self.registry.remove_server(name);
        self.save();
    }

    /// Connect enabled HTTP MCP servers and refresh tool lists.
    pub async fn connect_enabled_servers(&mut self) {
        let configs: Vec<_> = self
            .registry
            .servers
            .iter()
            .map(|server| server.config.clone())
            .collect();

        for config in configs {
            if !config.enabled {
                self.registry
                    .set_status(&config.name, McpServerStatus::Disconnected);
                continue;
            }

            self.registry
                .set_status(&config.name, McpServerStatus::Connecting);

            let declared = config.declared_tools.clone();
            match connect_mcp_server(&config).await {
                Ok((status, remote_tools)) => {
                    let tools = tools_for_server(&config.name, &declared, remote_tools);
                    self.registry.set_status(&config.name, status);
                    self.registry.set_tools(&config.name, tools);
                    self.last_error = None;
                }
                Err(error) => {
                    if !declared.is_empty() {
                        let tools = tools_for_server(&config.name, &declared, Vec::new());
                        self.registry
                            .set_status(&config.name, McpServerStatus::Connected);
                        self.registry.set_tools(&config.name, tools);
                        self.last_error = Some(format!(
                            "MCP '{}' live discovery failed; using declared tools: {}",
                            config.name, error
                        ));
                    } else {
                        self.registry
                            .set_status(&config.name, McpServerStatus::Error(error.to_string()));
                        self.registry.set_tools(&config.name, Vec::new());
                        self.last_error = Some(format!(
                            "MCP '{}' connection failed: {}",
                            config.name, error
                        ));
                    }
                }
            }
        }
        self.save();
    }

    /// Number of connected servers.
    pub fn connected_count(&self) -> usize {
        self.registry
            .servers
            .iter()
            .filter(|s| s.status == McpServerStatus::Connected)
            .count()
    }

    /// Total MCP tools available.
    pub fn tool_count(&self) -> usize {
        self.registry.all_tools().len()
    }
}
