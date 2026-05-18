use anyhow::Result;
use deepseek_mobile_core::{WorkspaceConnection, WorkspaceConnectionStore};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobileRuntimeConfig {
    pub thread_id: String,
    pub runtime_store_root: PathBuf,
    pub workspace_root: PathBuf,
    pub workspace_connection: Option<WorkspaceConnection>,
}

impl MobileRuntimeConfig {
    pub fn new(
        thread_id: impl Into<String>,
        runtime_store_root: impl Into<PathBuf>,
        workspace_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            thread_id: thread_id.into(),
            runtime_store_root: runtime_store_root.into(),
            workspace_root: workspace_root.into(),
            workspace_connection: None,
        }
    }

    pub fn from_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        Self::new(
            "mobile-default-thread",
            base_dir.join("runtime_store"),
            base_dir.join("workspace"),
        )
    }

    pub fn default_mobile() -> Self {
        Self::from_base_dir_with_saved_workspace(default_data_dir())
    }

    pub fn from_base_dir_with_saved_workspace(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        let mut config = Self::from_base_dir(base_dir.clone());
        if let Ok(Some(connection)) = load_active_workspace_connection_from_base_dir(&base_dir) {
            config.workspace_connection = Some(connection);
        }
        config
    }

    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = thread_id.into();
        self
    }

    pub fn with_workspace_connection(mut self, connection: WorkspaceConnection) -> Self {
        self.workspace_connection = Some(connection);
        self
    }

    pub fn runtime_store_root_display(&self) -> String {
        self.runtime_store_root.display().to_string()
    }

    pub fn session_file_path(&self) -> PathBuf {
        self.runtime_store_root.join("session.json")
    }

    pub fn workspace_root_display(&self) -> String {
        self.workspace_connection
            .as_ref()
            .map(|connection| connection.workspace_root.display().to_string())
            .unwrap_or_else(|| self.workspace_root.display().to_string())
    }
}

pub fn default_data_dir() -> PathBuf {
    std::env::var("DEEPSEEK_MOBILE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".deepseek-mobile"))
}

pub fn workspace_connection_store_for_base_dir(
    base_dir: impl AsRef<Path>,
) -> WorkspaceConnectionStore {
    WorkspaceConnectionStore::new(base_dir.as_ref().join("workspace_connections.json"))
}

pub fn default_workspace_connection_store() -> WorkspaceConnectionStore {
    workspace_connection_store_for_base_dir(default_data_dir())
}

pub fn load_saved_active_workspace_connection() -> Result<Option<WorkspaceConnection>> {
    load_active_workspace_connection_from_base_dir(default_data_dir())
}

pub fn load_active_workspace_connection_from_base_dir(
    base_dir: impl AsRef<Path>,
) -> Result<Option<WorkspaceConnection>> {
    let store = workspace_connection_store_for_base_dir(base_dir);
    let manager = store.load_or_default()?;
    Ok(manager.active().cloned())
}

pub fn activate_default_workspace_connection(connection: WorkspaceConnection) -> Result<()> {
    activate_workspace_connection_in_base_dir(default_data_dir(), connection)
}

pub fn activate_workspace_connection_in_base_dir(
    base_dir: impl AsRef<Path>,
    connection: WorkspaceConnection,
) -> Result<()> {
    let store = workspace_connection_store_for_base_dir(base_dir);
    let mut manager = store.load_or_default()?;
    let connection_id = connection.id.clone();
    manager.add_or_update(connection);
    manager.set_active(connection_id)?;
    store.save(&manager)
}

impl Default for MobileRuntimeConfig {
    fn default() -> Self {
        Self::default_mobile()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        activate_workspace_connection_in_base_dir, load_active_workspace_connection_from_base_dir,
        MobileRuntimeConfig,
    };
    use deepseek_mobile_core::{PcGatewayConfig, WorkspaceConnection};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_config_uses_mobile_thread() {
        let config = MobileRuntimeConfig::from_base_dir(temp_dir("default"));
        assert_eq!(config.thread_id, "mobile-default-thread");
        assert!(config
            .runtime_store_root_display()
            .contains("runtime_store"));
        assert!(config.workspace_root_display().contains("workspace"));
    }

    #[test]
    fn thread_id_can_be_overridden() {
        let config =
            MobileRuntimeConfig::from_base_dir("/tmp/deepseek-mobile").with_thread_id("thread-2");
        assert_eq!(config.thread_id, "thread-2");
    }

    #[test]
    fn pc_workspace_connection_overrides_workspace_display_root() {
        let gateway = PcGatewayConfig::new("pc-1", "Laptop", "http://127.0.0.1:8787", "phone-1");
        let connection = WorkspaceConnection::pc_gateway(
            "pc",
            "Laptop",
            "w1",
            "Project",
            "/pc/project",
            gateway,
        );
        let config =
            MobileRuntimeConfig::from_base_dir("/phone").with_workspace_connection(connection);
        assert_eq!(config.workspace_root_display(), "/pc/project");
    }

    #[test]
    fn saved_active_workspace_is_loaded_into_runtime_config() {
        let base_dir = temp_dir("saved-active");
        let gateway = PcGatewayConfig::new("pc-1", "Laptop", "http://127.0.0.1:8787", "phone-1");
        let connection = WorkspaceConnection::pc_gateway(
            "pc",
            "Laptop",
            "w1",
            "Project",
            "/pc/project",
            gateway,
        );
        activate_workspace_connection_in_base_dir(&base_dir, connection.clone()).unwrap();

        let loaded = load_active_workspace_connection_from_base_dir(&base_dir)
            .unwrap()
            .unwrap();
        let config = MobileRuntimeConfig::from_base_dir_with_saved_workspace(&base_dir);

        assert_eq!(loaded.id, "pc");
        assert_eq!(config.workspace_connection.as_ref().unwrap().id, "pc");
        assert_eq!(config.workspace_root_display(), "/pc/project");

        let _ = fs::remove_dir_all(base_dir);
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-mobile-runtime-config-{}-{}-{}",
            label,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
