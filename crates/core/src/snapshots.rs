//! Workspace snapshots and rollback.
//!
//! DeepSeek TUI uses side-git snapshots for terminal workspaces. Mobile cannot
//! assume a git binary or a writable global home directory, so this module uses
//! a portable file-copy snapshot store. It never writes inside the user's repo
//! unless the caller explicitly chooses a store path there, and restore is bound
//! to the workspace root.

use crate::workspace::Workspace;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const FILES_DIR: &str = "files";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSnapshotRecord {
    pub schema_version: u32,
    pub id: String,
    pub workspace_id: String,
    pub workspace_name: String,
    pub workspace_root: PathBuf,
    pub reason: String,
    pub created_unix: u64,
    pub file_count: usize,
    pub total_bytes: u64,
    pub files: Vec<WorkspaceSnapshotFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSnapshotFile {
    pub path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceRestoreReport {
    pub snapshot_id: String,
    pub restored_files: usize,
    pub removed_files: usize,
    pub skipped_files: usize,
}

#[derive(Clone, Debug)]
pub struct WorkspaceSnapshotService {
    workspace: Workspace,
    store_root: PathBuf,
}

impl WorkspaceSnapshotService {
    pub fn new(workspace: Workspace, store_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace,
            store_root: store_root.into(),
        }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn store_root(&self) -> &Path {
        &self.store_root
    }

    pub fn create_snapshot(&self, reason: impl Into<String>) -> Result<WorkspaceSnapshotRecord> {
        let created_unix = unix_time();
        let id = format!(
            "{}-{}",
            sanitize_component(&self.workspace.id),
            created_unix
        );
        let snapshot_dir = self.snapshot_dir(&id);
        let files_dir = snapshot_dir.join(FILES_DIR);
        fs::create_dir_all(&files_dir)
            .with_context(|| format!("create snapshot directory {}", files_dir.display()))?;

        let mut files = Vec::new();
        let mut total_bytes = 0u64;
        self.copy_workspace_tree(
            &self.workspace.root,
            &files_dir,
            &mut files,
            &mut total_bytes,
        )?;
        files.sort_by(|left, right| left.path.cmp(&right.path));

        let record = WorkspaceSnapshotRecord {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            id: id.clone(),
            workspace_id: self.workspace.id.clone(),
            workspace_name: self.workspace.name.clone(),
            workspace_root: self.workspace.root.clone(),
            reason: reason.into(),
            created_unix,
            file_count: files.len(),
            total_bytes,
            files,
        };
        write_json(&snapshot_dir.join(MANIFEST_FILE), &record)?;
        Ok(record)
    }

    pub fn list_snapshots(&self) -> Result<Vec<WorkspaceSnapshotRecord>> {
        let root = self.workspace_snapshot_root();
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in
            fs::read_dir(&root).with_context(|| format!("read snapshot root {}", root.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest = entry.path().join(MANIFEST_FILE);
            if manifest.exists() {
                let record = read_manifest(&manifest)?;
                records.push(record);
            }
        }
        records.sort_by_key(|r| std::cmp::Reverse(r.created_unix));
        Ok(records)
    }

    pub fn load_snapshot(&self, snapshot_id: &str) -> Result<WorkspaceSnapshotRecord> {
        let manifest = self.snapshot_dir(snapshot_id).join(MANIFEST_FILE);
        read_manifest(&manifest)
    }

    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<WorkspaceRestoreReport> {
        let record = self.load_snapshot(snapshot_id)?;
        if record.workspace_id != self.workspace.id {
            return Err(anyhow!(
                "snapshot workspace mismatch: expected {}, got {}",
                self.workspace.id,
                record.workspace_id
            ));
        }

        let snapshot_files_dir = self.snapshot_dir(snapshot_id).join(FILES_DIR);
        if !snapshot_files_dir.is_dir() {
            return Err(anyhow!(
                "snapshot files directory is missing: {}",
                snapshot_files_dir.display()
            ));
        }

        let snapshot_paths = record
            .files
            .iter()
            .map(|file| normalize_relative_path(&file.path))
            .collect::<Result<BTreeSet<_>>>()?;

        let mut restored_files = 0usize;
        let mut skipped_files = 0usize;
        for file in &record.files {
            let relative = normalize_relative_path(&file.path)?;
            let source = snapshot_files_dir.join(&relative);
            let target = self.safe_workspace_path(&relative)?;
            if !source.is_file() {
                skipped_files += 1;
                continue;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source, &target)
                .with_context(|| format!("restore {} to {}", source.display(), target.display()))?;
            restored_files += 1;
        }

        let mut removed_files = 0usize;
        for existing in self.collect_workspace_files()? {
            if self.should_skip_path(&self.workspace.root.join(&existing)) {
                continue;
            }
            if !snapshot_paths.contains(&existing) {
                let target = self.safe_workspace_path(&existing)?;
                if target.is_file() {
                    fs::remove_file(&target).with_context(|| {
                        format!("remove file not present in snapshot {}", target.display())
                    })?;
                    removed_files += 1;
                }
            }
        }
        prune_empty_dirs(&self.workspace.root, &self.workspace.root)?;

        Ok(WorkspaceRestoreReport {
            snapshot_id: snapshot_id.to_string(),
            restored_files,
            removed_files,
            skipped_files,
        })
    }

    fn workspace_snapshot_root(&self) -> PathBuf {
        self.store_root.join(sanitize_component(&self.workspace.id))
    }

    fn snapshot_dir(&self, snapshot_id: &str) -> PathBuf {
        self.workspace_snapshot_root()
            .join(sanitize_component(snapshot_id))
    }

    fn copy_workspace_tree(
        &self,
        current: &Path,
        files_dir: &Path,
        files: &mut Vec<WorkspaceSnapshotFile>,
        total_bytes: &mut u64,
    ) -> Result<()> {
        if self.should_skip_path(current) {
            return Ok(());
        }
        for entry in fs::read_dir(current)
            .with_context(|| format!("read workspace dir {}", current.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if self.should_skip_path(&path) {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                self.copy_workspace_tree(&path, files_dir, files, total_bytes)?;
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(&self.workspace.root)
                    .map_err(|_| anyhow!("file is outside workspace: {}", path.display()))?
                    .to_path_buf();
                let normalized = normalize_relative_path(&relative)?;
                let target = files_dir.join(&normalized);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&path, &target).with_context(|| {
                    format!("snapshot copy {} to {}", path.display(), target.display())
                })?;
                *total_bytes += metadata.len();
                files.push(WorkspaceSnapshotFile {
                    path: normalized,
                    size_bytes: metadata.len(),
                });
            }
        }
        Ok(())
    }

    fn collect_workspace_files(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        self.collect_workspace_files_from(&self.workspace.root, &mut out)?;
        out.sort();
        Ok(out)
    }

    fn collect_workspace_files_from(&self, current: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        if self.should_skip_path(current) {
            return Ok(());
        }
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if self.should_skip_path(&path) {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                self.collect_workspace_files_from(&path, out)?;
            } else if metadata.is_file() {
                out.push(
                    path.strip_prefix(&self.workspace.root)
                        .map_err(|_| anyhow!("file is outside workspace: {}", path.display()))?
                        .to_path_buf(),
                );
            }
        }
        Ok(())
    }

    fn safe_workspace_path(&self, relative: &Path) -> Result<PathBuf> {
        let relative = normalize_relative_path(relative)?;
        self.workspace
            .resolve_relative_path(&relative)
            .ok_or_else(|| anyhow!("snapshot path escapes workspace: {}", relative.display()))
    }

    /// Prune old snapshots, keeping at most `max_count` most recent entries
    /// and removing any snapshot older than `max_age_secs`.
    /// Returns the ids of removed snapshots.
    pub fn prune_old_snapshots(&self, max_count: usize, max_age_secs: u64) -> Result<Vec<String>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut snapshots = self.list_snapshots()?;
        if snapshots.len() <= max_count {
            return Ok(Vec::new());
        }
        snapshots.sort_by_key(|s| s.created_unix);

        let mut removed = Vec::new();
        for snapshot in &snapshots {
            if snapshots.len() - removed.len() <= max_count {
                break;
            }
            let age = now.saturating_sub(snapshot.created_unix);
            if age >= max_age_secs {
                let dir = self.snapshot_dir(&snapshot.id);
                if dir.exists() {
                    fs::remove_dir_all(&dir).with_context(|| {
                        format!("remove pruned snapshot directory {}", dir.display())
                    })?;
                }
                removed.push(snapshot.id.clone());
            }
        }
        Ok(removed)
    }

    fn should_skip_path(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if matches!(
            name,
            ".git" | "target" | "node_modules" | ".deepseek" | ".deepseek-mobile"
        ) {
            return true;
        }
        if let (Ok(path), Ok(store)) = (path.canonicalize(), self.store_root.canonicalize()) {
            path.starts_with(store)
        } else {
            false
        }
    }
}

fn read_manifest(path: &Path) -> Result<WorkspaceSnapshotRecord> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("read snapshot manifest {}", path.display()))?;
    let record: WorkspaceSnapshotRecord = serde_json::from_str(&text)?;
    if record.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported snapshot schema version: {}",
            record.schema_version
        ));
    }
    Ok(record)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Err(anyhow!(
            "absolute snapshot paths are not allowed: {}",
            path.display()
        ));
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("unsafe snapshot path: {}", path.display()));
            }
        }
    }
    Ok(out)
}

fn sanitize_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "snapshot".to_string()
    } else {
        sanitized
    }
}

fn prune_empty_dirs(root: &Path, current: &Path) -> Result<bool> {
    let mut is_empty = true;
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if !prune_empty_dirs(root, &path)? {
                is_empty = false;
            }
        } else {
            is_empty = false;
        }
    }
    if current != root && is_empty {
        fs::remove_dir(current)?;
    }
    Ok(is_empty)
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::WorkspaceSnapshotService;
    use crate::workspace::{ExecutorKind, Workspace};
    use std::fs;

    fn test_paths(name: &str) -> (Workspace, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "deepseek_mobile_snapshot_test_{}_{}",
            name,
            std::process::id()
        ));
        let workspace_root = base.join("workspace");
        let store_root = base.join("snapshots");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&workspace_root).unwrap();
        (
            Workspace::new(
                "w1",
                "Workspace",
                workspace_root,
                ExecutorKind::LocalAndroid,
            ),
            store_root,
        )
    }

    #[test]
    fn creates_and_lists_snapshot() {
        let (workspace, store_root) = test_paths("list");
        fs::write(workspace.root.join("README.md"), "v1").unwrap();
        let service = WorkspaceSnapshotService::new(workspace.clone(), store_root.clone());

        let snapshot = service.create_snapshot("before edit").unwrap();
        assert_eq!(snapshot.file_count, 1);
        assert_eq!(service.list_snapshots().unwrap().len(), 1);

        let _ = fs::remove_dir_all(store_root.parent().unwrap());
    }

    #[test]
    fn restores_changed_and_deleted_files() {
        let (workspace, store_root) = test_paths("restore");
        fs::create_dir_all(workspace.root.join("src")).unwrap();
        fs::write(workspace.root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let service = WorkspaceSnapshotService::new(workspace.clone(), store_root.clone());
        let snapshot = service.create_snapshot("before destructive edit").unwrap();

        fs::write(workspace.root.join("src/main.rs"), "broken").unwrap();
        fs::write(workspace.root.join("extra.txt"), "delete me").unwrap();

        let report = service.restore_snapshot(&snapshot.id).unwrap();
        assert_eq!(report.restored_files, 1);
        assert_eq!(
            fs::read_to_string(workspace.root.join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
        assert!(!workspace.root.join("extra.txt").exists());

        let _ = fs::remove_dir_all(store_root.parent().unwrap());
    }
}
