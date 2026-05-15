//! Workspace connection manager.
//!
//! A project can be driven from several execution backends: local Android
//! storage, Termux, a paired PC gateway, or a remote Y-lit runtime. The mobile
//! UI should not hard-code those choices. This module stores connection profiles,
//! tracks status, chooses the active backend, and converts the selected profile
//! into a `Workspace` boundary for tools and the engine.

use crate::pc_gateway::PcGatewayConfig;
use crate::workspace::{ExecutorKind, Workspace};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkspaceBackendKind {
    LocalAndroid,
    Termux,
    PcGateway,
    RemoteYlit,
}

impl WorkspaceBackendKind {
    pub fn executor_kind(&self) -> ExecutorKind {
        match self {
            WorkspaceBackendKind::LocalAndroid => ExecutorKind::LocalAndroid,
            WorkspaceBackendKind::Termux => ExecutorKind::Termux,
            WorkspaceBackendKind::PcGateway => ExecutorKind::PcGateway,
            WorkspaceBackendKind::RemoteYlit => ExecutorKind::RemoteYlit,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkspaceConnectionStatus {
    Unknown,
    SetupRequired,
    PairingRequired,
    Online,
    Offline,
    Unauthorized,
    Degraded(String),
    Error(String),
}

impl WorkspaceConnectionStatus {
    pub fn is_usable(&self) -> bool {
        matches!(self, WorkspaceConnectionStatus::Online | WorkspaceConnectionStatus::Degraded(_))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConnection {
    pub id: String,
    pub label: String,
    pub backend: WorkspaceBackendKind,
    pub workspace_id: String,
    pub workspace_name: String,
    pub workspace_root: PathBuf,
    pub pc_gateway: Option<PcGatewayConfig>,
    pub remote_base_url: Option<String>,
    pub environment_id: Option<String>,
    pub status: WorkspaceConnectionStatus,
    pub priority: u32,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl WorkspaceConnection {
    pub fn local_android(
        id: impl Into<String>,
        label: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_name: impl Into<String>,
        root: impl Into<PathBuf>,
    ) -> Self {
        Self::new(
            id,
            label,
            WorkspaceBackendKind::LocalAndroid,
            workspace_id,
            workspace_name,
            root,
        )
        .with_status(WorkspaceConnectionStatus::Online)
    }

    pub fn termux(
        id: impl Into<String>,
        label: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_name: impl Into<String>,
        root: impl Into<PathBuf>,
    ) -> Self {
        Self::new(id, label, WorkspaceBackendKind::Termux, workspace_id, workspace_name, root)
            .with_status(WorkspaceConnectionStatus::SetupRequired)
    }

    pub fn pc_gateway(
        id: impl Into<String>,
        label: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_name: impl Into<String>,
        root: impl Into<PathBuf>,
        gateway: PcGatewayConfig,
    ) -> Self {
        let mut connection = Self::new(
            id,
            label,
            WorkspaceBackendKind::PcGateway,
            workspace_id,
            workspace_name,
            root,
        );
        connection.pc_gateway = Some(gateway);
        connection.status = WorkspaceConnectionStatus::PairingRequired;
        connection.priority = 100;
        connection
    }

    pub fn remote_ylit(
        id: impl Into<String>,
        label: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_name: impl Into<String>,
        root: impl Into<PathBuf>,
        base_url: impl Into<String>,
    ) -> Self {
        let mut connection = Self::new(
            id,
            label,
            WorkspaceBackendKind::RemoteYlit,
            workspace_id,
            workspace_name,
            root,
        );
        connection.remote_base_url = Some(base_url.into());
        connection.status = WorkspaceConnectionStatus::Unknown;
        connection.priority = 80;
        connection
    }

    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        backend: WorkspaceBackendKind,
        workspace_id: impl Into<String>,
        workspace_name: impl Into<String>,
        root: impl Into<PathBuf>,
    ) -> Self {
        let now = current_unix_time();
        Self {
            id: id.into(),
            label: label.into(),
            backend,
            workspace_id: workspace_id.into(),
            workspace_name: workspace_name.into(),
            workspace_root: root.into(),
            pc_gateway: None,
            remote_base_url: None,
            environment_id: None,
            status: WorkspaceConnectionStatus::Unknown,
            priority: 10,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    pub fn with_status(mut self, status: WorkspaceConnectionStatus) -> Self {
        self.status = status;
        self.updated_at_unix = current_unix_time();
        self
    }

    pub fn with_environment(mut self, environment_id: impl Into<String>) -> Self {
        self.environment_id = Some(environment_id.into());
        self.updated_at_unix = current_unix_time();
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self.updated_at_unix = current_unix_time();
        self
    }

    pub fn is_usable(&self) -> bool {
        self.status.is_usable()
    }

    pub fn to_workspace(&self) -> Workspace {
        Workspace::new(
            self.workspace_id.clone(),
            self.workspace_name.clone(),
            self.workspace_root.clone(),
            self.backend.executor_kind(),
        )
    }

    pub fn mark_status(&mut self, status: WorkspaceConnectionStatus) {
        self.status = status;
        self.updated_at_unix = current_unix_time();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConnectionManager {
    pub connections: Vec<WorkspaceConnection>,
    pub active_connection_id: Option<String>,
}

impl WorkspaceConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_or_update(&mut self, connection: WorkspaceConnection) {
        if let Some(existing) = self.connections.iter_mut().find(|item| item.id == connection.id) {
            *existing = connection;
        } else {
            self.connections.push(connection);
        }
    }

    pub fn remove(&mut self, connection_id: &str) -> Option<WorkspaceConnection> {
        let position = self.connections.iter().position(|item| item.id == connection_id)?;
        if self.active_connection_id.as_deref() == Some(connection_id) {
            self.active_connection_id = None;
        }
        Some(self.connections.remove(position))
    }

    pub fn set_active(&mut self, connection_id: impl Into<String>) -> Result<()> {
        let connection_id = connection_id.into();
        if !self.connections.iter().any(|item| item.id == connection_id) {
            return Err(anyhow!("workspace connection not found: {}", connection_id));
        }
        self.active_connection_id = Some(connection_id);
        Ok(())
    }

    pub fn active(&self) -> Option<&WorkspaceConnection> {
        self.active_connection_id
            .as_deref()
            .and_then(|id| self.connections.iter().find(|item| item.id == id))
    }

    pub fn active_workspace(&self) -> Option<Workspace> {
        self.active().map(WorkspaceConnection::to_workspace)
    }

    pub fn get(&self, connection_id: &str) -> Option<&WorkspaceConnection> {
        self.connections.iter().find(|item| item.id == connection_id)
    }

    pub fn get_mut(&mut self, connection_id: &str) -> Option<&mut WorkspaceConnection> {
        self.connections.iter_mut().find(|item| item.id == connection_id)
    }

    pub fn mark_status(&mut self, connection_id: &str, status: WorkspaceConnectionStatus) -> Result<()> {
        let connection = self
            .get_mut(connection_id)
            .ok_or_else(|| anyhow!("workspace connection not found: {}", connection_id))?;
        connection.mark_status(status);
        Ok(())
    }

    pub fn list_for_workspace(&self, workspace_id: &str) -> Vec<&WorkspaceConnection> {
        self.connections
            .iter()
            .filter(|item| item.workspace_id == workspace_id)
            .collect()
    }

    pub fn best_usable_for_workspace(&self, workspace_id: &str) -> Option<&WorkspaceConnection> {
        self.connections
            .iter()
            .filter(|item| item.workspace_id == workspace_id && item.is_usable())
            .max_by_key(|item| item.priority)
    }

    pub fn choose_best_active(&mut self, workspace_id: &str) -> Option<&WorkspaceConnection> {
        let best_id = self
            .best_usable_for_workspace(workspace_id)
            .map(|connection| connection.id.clone())?;
        self.active_connection_id = Some(best_id);
        self.active()
    }
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceBackendKind, WorkspaceConnection, WorkspaceConnectionManager, WorkspaceConnectionStatus};
    use crate::pc_gateway::PcGatewayConfig;
    use crate::workspace::ExecutorKind;
    use std::path::PathBuf;

    #[test]
    fn converts_connection_to_workspace() {
        let connection = WorkspaceConnection::local_android(
            "local-1",
            "Phone",
            "workspace-1",
            "Project",
            PathBuf::from("/phone/project"),
        );
        let workspace = connection.to_workspace();
        assert_eq!(workspace.executor, ExecutorKind::LocalAndroid);
        assert_eq!(workspace.id, "workspace-1");
    }

    #[test]
    fn chooses_highest_priority_usable_backend() {
        let mut manager = WorkspaceConnectionManager::new();
        manager.add_or_update(
            WorkspaceConnection::local_android("local", "Phone", "w1", "Project", "/phone/project")
                .with_priority(10),
        );
        let mut gateway = PcGatewayConfig::new("pc-1", "Laptop", "https://pc.local", "phone-1");
        gateway.auth_token = Some("token".to_string());
        manager.add_or_update(
            WorkspaceConnection::pc_gateway("pc", "Laptop", "w1", "Project", "/pc/project", gateway)
                .with_status(WorkspaceConnectionStatus::Online)
                .with_priority(100),
        );

        let best = manager.choose_best_active("w1").unwrap();
        assert_eq!(best.backend, WorkspaceBackendKind::PcGateway);
        assert_eq!(manager.active().unwrap().id, "pc");
    }

    #[test]
    fn rejects_missing_active_connection() {
        let mut manager = WorkspaceConnectionManager::new();
        assert!(manager.set_active("missing").is_err());
    }
}
