//! Workspace model for mobile and remote projects.
//!
//! A workspace is the boundary where the agent is allowed to read, write,
//! patch and execute project-specific commands. Keeping this boundary explicit
//! is essential for Android scoped storage, Termux integration, PC companion
//! gateways and remote execution backends.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub executor: ExecutorKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutorKind {
    LocalAndroid,
    Termux,
    PcGateway,
    RemoteYlit,
}

impl Workspace {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        root: impl Into<PathBuf>,
        executor: ExecutorKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            root: root.into(),
            executor,
        }
    }

    pub fn resolve_relative_path(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();

        if path.is_absolute() {
            return path.starts_with(&self.root).then(|| path.to_path_buf());
        }

        let mut safe_relative = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => safe_relative.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
            }
        }

        Some(self.root.join(safe_relative))
    }

    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        self.resolve_relative_path(path)
            .map(|resolved| resolved.starts_with(&self.root))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutorKind, Workspace};
    use std::path::PathBuf;

    fn workspace() -> Workspace {
        Workspace::new(
            "test",
            "Test Workspace",
            PathBuf::from("/safe/workspace"),
            ExecutorKind::LocalAndroid,
        )
    }

    #[test]
    fn accepts_safe_relative_paths() {
        let workspace = workspace();
        assert!(workspace.contains("src/main.rs"));
        assert!(workspace.contains("./README.md"));
    }

    #[test]
    fn rejects_parent_directory_traversal() {
        let workspace = workspace();
        assert!(!workspace.contains("../secrets.txt"));
        assert!(!workspace.contains("src/../../secrets.txt"));
    }

    #[test]
    fn accepts_absolute_paths_inside_workspace() {
        let workspace = workspace();
        assert!(workspace.contains("/safe/workspace/src/lib.rs"));
    }

    #[test]
    fn rejects_absolute_paths_outside_workspace() {
        let workspace = workspace();
        assert!(!workspace.contains("/safe/other/lib.rs"));
        assert!(!workspace.contains("/etc/passwd"));
    }
}