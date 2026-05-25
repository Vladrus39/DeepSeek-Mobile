//! Persistent store for workspace connection profiles.
//!
//! The mobile app must remember known backends across restarts, but it should
//! not silently switch to a PC or remote backend unless the user selected that
//! policy. This store persists the `WorkspaceConnectionManager` exactly as the
//! user configured it.

use crate::workspace_connection::{WorkspaceConnectionManager, WorkspaceSelectionPolicy};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CURRENT_CONNECTION_STORE_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConnectionStoreFile {
    pub schema_version: u32,
    pub manager: WorkspaceConnectionManager,
}

impl WorkspaceConnectionStoreFile {
    pub fn new(manager: WorkspaceConnectionManager) -> Self {
        Self {
            schema_version: CURRENT_CONNECTION_STORE_VERSION,
            manager,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceConnectionStore {
    path: PathBuf,
}

impl WorkspaceConnectionStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_default(&self) -> Result<WorkspaceConnectionManager> {
        if !self.path.exists() {
            let manager = WorkspaceConnectionManager::new()
                .with_selection_policy(WorkspaceSelectionPolicy::PreferTermux);
            self.save(&manager)?;
            return Ok(manager);
        }

        let raw = fs::read_to_string(&self.path)
            .with_context(|| format!("read {}", self.path.display()))?;
        let file: WorkspaceConnectionStoreFile = serde_json::from_str(&raw)
            .with_context(|| format!("parse {}", self.path.display()))?;

        if file.schema_version > CURRENT_CONNECTION_STORE_VERSION {
            anyhow::bail!(
                "workspace connection store schema v{} is newer than supported v{}",
                file.schema_version,
                CURRENT_CONNECTION_STORE_VERSION
            );
        }

        Ok(file.manager)
    }

    pub fn save(&self, manager: &WorkspaceConnectionManager) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let file = WorkspaceConnectionStoreFile::new(manager.clone());
        let tmp_path = self.path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(&file)?;
        fs::write(&tmp_path, raw).with_context(|| format!("write {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &self.path)
            .with_context(|| format!("rename {} -> {}", tmp_path.display(), self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceConnectionStore;
    use crate::workspace_connection::{
        WorkspaceConnection, WorkspaceConnectionManager, WorkspaceSelectionPolicy,
    };
    use std::fs;

    fn store(name: &str) -> WorkspaceConnectionStore {
        let path = std::env::temp_dir().join(format!(
            "deepseek_mobile_workspace_connections_{}_{}.json",
            name,
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        WorkspaceConnectionStore::new(path)
    }

    #[test]
    fn creates_default_manual_store() {
        let store = store("default");
        let manager = store.load_or_default().unwrap();
        assert_eq!(manager.selection_policy, WorkspaceSelectionPolicy::PreferTermux);
        assert!(store.path().exists());
        let _ = fs::remove_file(store.path());
    }

    #[test]
    fn saves_and_loads_connections() {
        let store = store("save_load");
        let mut manager = WorkspaceConnectionManager::new()
            .with_selection_policy(WorkspaceSelectionPolicy::PreferLocal);
        manager.add_or_update(WorkspaceConnection::local_android(
            "local",
            "Phone",
            "w1",
            "Project",
            "/phone/project",
        ));
        manager.set_active("local").unwrap();
        store.save(&manager).unwrap();

        let loaded = store.load_or_default().unwrap();
        assert_eq!(loaded.selection_policy, WorkspaceSelectionPolicy::PreferLocal);
        assert_eq!(loaded.active().unwrap().id, "local");
        let _ = fs::remove_file(store.path());
    }
}
