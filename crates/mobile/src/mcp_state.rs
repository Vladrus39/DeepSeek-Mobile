use deepseek_mobile_core::{McpClientRegistry, McpServerStatus};
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
