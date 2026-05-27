//! Safe workspace file operations.
//!
//! These operations are intentionally bound to a `Workspace`. The agent should
//! never read or write arbitrary Android paths. All file operations must pass
//! through workspace boundary checks first.

use crate::workspace::Workspace;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceFileEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size_bytes: u64,
}

pub struct WorkspaceFileService {
    workspace: Workspace,
}

impl WorkspaceFileService {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn list_files(&self, relative_dir: impl AsRef<Path>) -> Result<Vec<WorkspaceFileEntry>> {
        let dir = self.resolve_existing_path(relative_dir)?;
        if !dir.is_dir() {
            return Err(anyhow!(
                "workspace path is not a directory: {}",
                dir.display()
            ));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let absolute_path = entry.path();
            let relative_path = absolute_path
                .strip_prefix(&self.workspace.root)
                .unwrap_or(&absolute_path)
                .to_path_buf();

            entries.push(WorkspaceFileEntry {
                path: relative_path,
                is_dir: metadata.is_dir(),
                size_bytes: metadata.len(),
            });
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    pub fn read_text_file(&self, relative_path: impl AsRef<Path>) -> Result<String> {
        let path = self.resolve_existing_path(relative_path)?;
        if !path.is_file() {
            return Err(anyhow!("workspace path is not a file: {}", path.display()));
        }

        Ok(fs::read_to_string(path)?)
    }

    pub fn write_text_file(&self, relative_path: impl AsRef<Path>, content: &str) -> Result<()> {
        let path = self.resolve_safe_path(relative_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    pub fn copy_file(&self, source: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
        let src = self.resolve_existing_path(source)?;
        if !src.is_file() {
            return Err(anyhow!("source is not a file: {}", src.display()));
        }
        let dst = self.resolve_safe_path(dest)?;
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&src, &dst)?;
        Ok(())
    }

    pub fn rename_file(&self, source: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
        let src = self.resolve_existing_path(source)?;
        if !src.is_file() {
            return Err(anyhow!("source is not a file: {}", src.display()));
        }
        let dst = self.resolve_safe_path(dest)?;
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&src, &dst)?;
        Ok(())
    }

    pub fn delete_file(&self, relative_path: impl AsRef<Path>) -> Result<()> {
        let path = self.resolve_existing_path(relative_path)?;
        if !path.is_file() {
            return Err(anyhow!("workspace path is not a file: {}", path.display()));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    fn resolve_safe_path(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        self.workspace
            .resolve_relative_path(relative_path)
            .ok_or_else(|| anyhow!("path is outside workspace boundary"))
    }

    fn resolve_existing_path(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = self.resolve_safe_path(relative_path)?;
        if !path.exists() {
            return Err(anyhow!("workspace path does not exist: {}", path.display()));
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceFileService;
    use crate::workspace::{ExecutorKind, Workspace};
    use std::fs;
    use std::path::PathBuf;

    fn test_workspace(name: &str) -> Workspace {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_workspace_files_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Workspace::new(name, name, root, ExecutorKind::LocalAndroid)
    }

    #[test]
    fn writes_and_reads_text_file_inside_workspace() {
        let workspace = test_workspace("read_write");
        let service = WorkspaceFileService::new(workspace.clone());

        service
            .write_text_file("src/main.rs", "fn main() {}")
            .unwrap();
        let content = service.read_text_file("src/main.rs").unwrap();

        assert_eq!(content, "fn main() {}");
        let _ = fs::remove_dir_all(workspace.root);
    }

    #[test]
    fn rejects_write_outside_workspace() {
        let workspace = test_workspace("reject_escape");
        let service = WorkspaceFileService::new(workspace.clone());

        let err = service
            .write_text_file("../outside.txt", "secret")
            .unwrap_err();
        assert!(err.to_string().contains("outside workspace"));
        let _ = fs::remove_dir_all(workspace.root);
    }

    #[test]
    fn lists_files_inside_workspace() {
        let workspace = test_workspace("list_files");
        let service = WorkspaceFileService::new(workspace.clone());

        service.write_text_file("a.txt", "a").unwrap();
        service.write_text_file("b.txt", "bb").unwrap();

        let entries = service.list_files(".").unwrap();
        let paths = entries
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<PathBuf>>();

        assert!(paths.contains(&PathBuf::from("a.txt")));
        assert!(paths.contains(&PathBuf::from("b.txt")));
        let _ = fs::remove_dir_all(workspace.root);
    }
}
