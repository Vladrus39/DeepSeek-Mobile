//! Workspace model for mobile and remote projects.
//!
//! A workspace is the boundary where the agent is allowed to read, write,
//! patch and execute project-specific commands. Keeping this boundary explicit
//! is essential for Android scoped storage, Termux integration and remote
//! execution backends.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if path.is_absolute() {
            path.starts_with(&self.root)
        } else {
            self.root.join(path).starts_with(&self.root)
        }
    }
}
