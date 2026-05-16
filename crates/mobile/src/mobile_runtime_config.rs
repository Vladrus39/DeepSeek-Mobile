use deepseek_mobile_core::WorkspaceConnection;
use std::path::PathBuf;

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
        if let Ok(base_dir) = std::env::var("DEEPSEEK_MOBILE_DATA_DIR") {
            return Self::from_base_dir(base_dir);
        }

        Self::from_base_dir(".deepseek-mobile")
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

    pub fn workspace_root_display(&self) -> String {
        self.workspace_connection
            .as_ref()
            .map(|connection| connection.workspace_root.display().to_string())
            .unwrap_or_else(|| self.workspace_root.display().to_string())
    }
}

impl Default for MobileRuntimeConfig {
    fn default() -> Self {
        Self::default_mobile()
    }
}

#[cfg(test)]
mod tests {
    use super::MobileRuntimeConfig;
    use deepseek_mobile_core::{PcGatewayConfig, WorkspaceConnection};

    #[test]
    fn default_config_uses_mobile_thread() {
        let config = MobileRuntimeConfig::default();
        assert_eq!(config.thread_id, "mobile-default-thread");
        assert!(config.runtime_store_root_display().contains("runtime_store"));
        assert!(config.workspace_root_display().contains("workspace"));
    }

    #[test]
    fn thread_id_can_be_overridden() {
        let config = MobileRuntimeConfig::from_base_dir("/tmp/deepseek-mobile").with_thread_id("thread-2");
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
        let config = MobileRuntimeConfig::from_base_dir("/phone").with_workspace_connection(connection);
        assert_eq!(config.workspace_root_display(), "/pc/project");
    }
}