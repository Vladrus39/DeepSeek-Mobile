//! MCP (Model Context Protocol) client configuration and tool registry.
//!
//! Supports two transports:
//!   - stdio: local MCP server spawned as a child process
//!   - http_sse: remote MCP server via HTTP/SSE
//!
//! Tools from connected MCP servers are merged into the agent tool loop
//! at runtime alongside built-in tools.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ── Server Config ──

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "transport")]
pub enum McpTransport {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
    },
    #[serde(rename = "http_sse")]
    HttpSse {
        url: String,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        headers: HashMap<String, String>,
    },
}

impl McpTransport {
    pub fn kind_str(&self) -> &'static str {
        match self {
            McpTransport::Stdio { .. } => "stdio",
            McpTransport::HttpSse { .. } => "http-sse",
        }
    }

    pub fn label(&self) -> String {
        match self {
            McpTransport::Stdio { command, args, .. } => {
                format!("{} {}", command, args.join(" "))
            }
            McpTransport::HttpSse { url, .. } => url.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpTransport,
    /// Whether this server should be auto-started
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional static tool list when live MCP discovery is unavailable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declared_tools: Vec<McpToolDescriptor>,
}

fn default_true() -> bool {
    true
}

// ── Tool Descriptor ──

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpToolDescriptor {
    pub name: String,
    pub server: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

// ── Server Status ──

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum McpServerStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl McpServerStatus {
    pub fn label(&self) -> &'static str {
        match self {
            McpServerStatus::Disconnected => "Disconnected",
            McpServerStatus::Connecting => "Connecting…",
            McpServerStatus::Connected => "Connected",
            McpServerStatus::Error(_) => "Error",
        }
    }
}

// ── Server State ──

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpServerState {
    pub config: McpServerConfig,
    pub status: McpServerStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<McpToolDescriptor>,
}

// ── Registry ──

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct McpClientRegistry {
    pub servers: Vec<McpServerState>,
}

impl McpClientRegistry {
    /// Load from `~/.deepseek-mobile/mcp.json`.
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path)?;
        let configs: Vec<McpServerConfig> = serde_json::from_slice(&bytes)
            .map_err(|e| anyhow!("failed to parse {}: {}", path.display(), e))?;
        let servers = configs
            .into_iter()
            .map(|c| McpServerState {
                config: c.clone(),
                status: if c.enabled {
                    McpServerStatus::Disconnected
                } else {
                    McpServerStatus::Disconnected
                },
                tools: Vec::new(),
            })
            .collect();
        Ok(Self { servers })
    }

    /// Save configs to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let configs: Vec<&McpServerConfig> = self.servers.iter().map(|s| &s.config).collect();
        fs::write(path, serde_json::to_string_pretty(&configs)?)?;
        Ok(())
    }

    /// Add a server config.
    pub fn add_server(&mut self, config: McpServerConfig) -> Result<()> {
        if self.servers.iter().any(|s| s.config.name == config.name) {
            return Err(anyhow!("server '{}' already exists", config.name));
        }
        self.servers.push(McpServerState {
            config: config.clone(),
            status: if config.enabled {
                McpServerStatus::Disconnected
            } else {
                McpServerStatus::Disconnected
            },
            tools: Vec::new(),
        });
        Ok(())
    }

    /// Remove a server by name.
    pub fn remove_server(&mut self, name: &str) -> bool {
        let len_before = self.servers.len();
        self.servers.retain(|s| s.config.name != name);
        self.servers.len() < len_before
    }

    /// Update server status.
    pub fn set_status(&mut self, name: &str, status: McpServerStatus) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            server.status = status;
        }
    }

    /// Update tool list for a server.
    pub fn set_tools(&mut self, name: &str, tools: Vec<McpToolDescriptor>) {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            server.tools = tools;
        }
    }

    /// Enable or disable a server.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(server) = self.servers.iter_mut().find(|s| s.config.name == name) {
            server.config.enabled = enabled;
            if !enabled {
                server.status = McpServerStatus::Disconnected;
                server.tools.clear();
            }
            true
        } else {
            false
        }
    }

    /// Collect all tools from connected servers for injection into the tool registry.
    pub fn all_tools(&self) -> Vec<McpToolDescriptor> {
        self.servers
            .iter()
            .filter(|s| s.status == McpServerStatus::Connected)
            .flat_map(|s| s.tools.clone())
            .collect()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_remove_server() {
        let mut registry = McpClientRegistry::default();
        let config = McpServerConfig {
            name: "test-server".to_string(),
            transport: McpTransport::Stdio {
                command: "node".to_string(),
                args: vec!["server.js".to_string()],
                env: HashMap::new(),
            },
            enabled: true,
            description: None,
            declared_tools: Vec::new(),
        };
        registry.add_server(config).unwrap();
        assert_eq!(registry.servers.len(), 1);

        assert!(registry.remove_server("test-server"));
        assert!(registry.servers.is_empty());
    }

    #[test]
    fn test_duplicate_server_rejected() {
        let mut registry = McpClientRegistry::default();
        let config = McpServerConfig {
            name: "dup".to_string(),
            transport: McpTransport::HttpSse {
                url: "http://localhost:3000/mcp".to_string(),
                headers: HashMap::new(),
            },
            enabled: true,
            description: None,
            declared_tools: Vec::new(),
        };
        registry.add_server(config.clone()).unwrap();
        assert!(registry.add_server(config).is_err());
    }

    #[test]
    fn test_set_status_and_tools() {
        let mut registry = McpClientRegistry::default();
        let config = McpServerConfig {
            name: "srv".to_string(),
            transport: McpTransport::Stdio {
                command: "cmd".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
            enabled: true,
            description: None,
            declared_tools: Vec::new(),
        };
        registry.add_server(config).unwrap();

        registry.set_status("srv", McpServerStatus::Connected);
        assert_eq!(registry.servers[0].status, McpServerStatus::Connected);

        let tools = vec![McpToolDescriptor {
            name: "fetch".to_string(),
            server: "srv".to_string(),
            description: Some("Fetch data".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        registry.set_tools("srv", tools);
        assert_eq!(registry.servers[0].tools.len(), 1);
        assert_eq!(registry.all_tools().len(), 1);
    }

    #[test]
    fn test_set_enabled_disconnects() {
        let mut registry = McpClientRegistry::default();
        let config = McpServerConfig {
            name: "srv".to_string(),
            transport: McpTransport::Stdio {
                command: "cmd".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
            enabled: true,
            description: None,
            declared_tools: Vec::new(),
        };
        registry.add_server(config).unwrap();
        registry.set_status("srv", McpServerStatus::Connected);
        registry.set_tools(
            "srv",
            vec![McpToolDescriptor {
                name: "t1".to_string(),
                server: "srv".to_string(),
                description: None,
                input_schema: serde_json::json!({}),
            }],
        );

        registry.set_enabled("srv", false);
        assert_eq!(registry.servers[0].status, McpServerStatus::Disconnected);
        assert!(registry.servers[0].tools.is_empty());
        assert!(registry.all_tools().is_empty());
    }

    #[test]
    fn test_load_save_roundtrip() {
        let dir = temp_dir();
        let path = dir.join("mcp.json");

        // Create and save
        let mut reg = McpClientRegistry::default();
        reg.add_server(McpServerConfig {
            name: "srv".to_string(),
            transport: McpTransport::HttpSse {
                url: "http://localhost:4000/mcp".to_string(),
                headers: HashMap::new(),
            },
            enabled: true,
            description: Some("Test server".to_string()),
            declared_tools: Vec::new(),
        })
        .unwrap();
        reg.save(&path).unwrap();

        // Load
        let loaded = McpClientRegistry::load_or_default(&path).unwrap();
        assert_eq!(loaded.servers.len(), 1);
        assert_eq!(loaded.servers[0].config.name, "srv");
        clean(&dir);
    }

    #[test]
    fn test_unknown_server_ops_noop() {
        let mut registry = McpClientRegistry::default();
        assert!(!registry.remove_server("ghost"));
        assert!(!registry.set_enabled("ghost", true));
        registry.set_status("ghost", McpServerStatus::Connected); // no-op
    }

    #[test]
    fn test_transport_label() {
        let stdio = McpTransport::Stdio {
            command: "python".to_string(),
            args: vec!["-m".to_string(), "mcp".to_string()],
            env: HashMap::new(),
        };
        assert_eq!(stdio.kind_str(), "stdio");
        assert!(stdio.label().contains("python"));

        let http = McpTransport::HttpSse {
            url: "http://host:3000/mcp".to_string(),
            headers: HashMap::new(),
        };
        assert_eq!(http.kind_str(), "http-sse");
        assert_eq!(http.label(), "http://host:3000/mcp");
    }

    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let pid = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-mcp-test-{}-{}", pid, ts))
    }

    fn clean(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }
}
