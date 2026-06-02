//! Durable task records with JSON persistence.
//!
//! Tracks background task lifecycle (queued → running → completed/failed/canceled)
//! on the mobile/core side, independent of individual PC-host task handles.
//! Survives app restart via a single JSON file under the runtime store root.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Status ──

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DurableTaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
}

impl DurableTaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DurableTaskStatus::Completed | DurableTaskStatus::Failed | DurableTaskStatus::Canceled
        )
    }
}

// ── Record ──

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DurableTaskRecord {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub status: DurableTaskStatus,

    pub created_at_unix: u64,
    pub started_at_unix: Option<u64>,
    pub completed_at_unix: Option<u64>,

    /// Paths to task output artifacts (logs, reports, screenshots, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_paths: Vec<String>,

    /// Human-readable error message when the task failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Short summary of the task result (e.g. "2 tests passed, 1 failed").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_summary: Option<String>,
}

impl DurableTaskRecord {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: kind.into(),
            status: DurableTaskStatus::Queued,
            created_at_unix: unix_secs(),
            started_at_unix: None,
            completed_at_unix: None,
            artifact_paths: Vec::new(),
            error_message: None,
            result_summary: None,
        }
    }

    pub fn mark_running(&mut self) {
        self.status = DurableTaskStatus::Running;
        self.started_at_unix = self.started_at_unix.or(Some(unix_secs()));
    }

    pub fn mark_completed(&mut self, result_summary: impl Into<String>) {
        self.status = DurableTaskStatus::Completed;
        self.completed_at_unix = Some(unix_secs());
        self.result_summary = Some(result_summary.into());
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = DurableTaskStatus::Failed;
        self.completed_at_unix = Some(unix_secs());
        self.error_message = Some(error.into());
    }

    pub fn mark_canceled(&mut self) {
        self.status = DurableTaskStatus::Canceled;
        self.completed_at_unix = Some(unix_secs());
    }

    /// Add an artifact path to this task record.
    pub fn add_artifact(&mut self, path: impl Into<String>) {
        self.artifact_paths.push(path.into());
    }

    /// Check whether this task has any artifacts.
    pub fn has_artifacts(&self) -> bool {
        !self.artifact_paths.is_empty()
    }

    pub fn artifact_count(&self) -> usize {
        self.artifact_paths.len()
    }
}

// ── Manager ──

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct DurableTaskManagerState {
    tasks: Vec<DurableTaskRecord>,
}

#[derive(Clone, Debug)]
pub struct DurableTaskManager {
    path: PathBuf,
}

impl DurableTaskManager {
    /// Open or create the manager at the given JSON file path.
    /// The parent directory is created if it does not exist.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    /// Open the manager under `base_dir/tasks.json`.
    pub fn open_in(base_dir: impl AsRef<Path>) -> Result<Self> {
        Self::open(base_dir.as_ref().join("tasks.json"))
    }

    /// Load all task records from the JSON file.
    /// Returns an empty list if the file does not exist yet.
    pub fn load_all(&self) -> Result<Vec<DurableTaskRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&self.path)?;
        let state: DurableTaskManagerState = serde_json::from_slice(&bytes)
            .map_err(|e| anyhow!("failed to parse {}: {}", self.path.display(), e))?;
        Ok(state.tasks)
    }

    /// Load a single task by id.
    pub fn load(&self, task_id: &str) -> Result<Option<DurableTaskRecord>> {
        Ok(self.load_all()?.into_iter().find(|t| t.id == task_id))
    }

    /// Save a task record (insert or update in place).
    pub fn save(&self, record: &DurableTaskRecord) -> Result<()> {
        let mut state = self.load_state_or_default()?;
        if let Some(existing) = state.tasks.iter_mut().find(|t| t.id == record.id) {
            *existing = record.clone();
        } else {
            state.tasks.push(record.clone());
        }
        self.write_state(&state)
    }

    /// Create a new task record with a unique id.
    /// Returns the created record.
    pub fn create(
        &self,
        id: impl Into<String>,
        label: impl Into<String>,
        kind: impl Into<String>,
    ) -> Result<DurableTaskRecord> {
        let record = DurableTaskRecord::new(id, label, kind);
        self.save(&record)?;
        Ok(record)
    }

    /// Update the status of an existing task.
    pub fn update_status(
        &self,
        task_id: &str,
        status: DurableTaskStatus,
    ) -> Result<Option<DurableTaskRecord>> {
        let mut state = self.load_state_or_default()?;
        let found = state.tasks.iter_mut().find(|t| t.id == task_id);
        match found {
            Some(record) => {
                let mut updated = record.clone();
                updated.status = status.clone();
                match &status {
                    DurableTaskStatus::Running => {
                        updated.started_at_unix = updated.started_at_unix.or(Some(unix_secs()))
                    }
                    DurableTaskStatus::Completed
                    | DurableTaskStatus::Failed
                    | DurableTaskStatus::Canceled => {
                        updated.completed_at_unix = Some(unix_secs());
                    }
                    DurableTaskStatus::Queued => {}
                }
                *record = updated.clone();
                self.write_state(&state)?;
                Ok(Some(updated))
            }
            None => Ok(None),
        }
    }

    /// Delete a task record by id.
    pub fn delete(&self, task_id: &str) -> Result<bool> {
        let mut state = self.load_state_or_default()?;
        let len_before = state.tasks.len();
        state.tasks.retain(|t| t.id != task_id);
        if state.tasks.len() < len_before {
            self.write_state(&state)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Count tasks by status.
    pub fn count_by_status(&self) -> Result<std::collections::HashMap<String, usize>> {
        let tasks = self.load_all()?;
        let mut counts = std::collections::HashMap::new();
        for task in &tasks {
            *counts.entry(format!("{:?}", task.status)).or_insert(0) += 1;
        }
        Ok(counts)
    }

    /// Remove all terminal tasks.
    pub fn prune_terminal_tasks(&self) -> Result<usize> {
        let mut state = self.load_state_or_default()?;
        let before = state.tasks.len();
        state.tasks.retain(|t| !t.status.is_terminal());
        let removed = before - state.tasks.len();
        if removed > 0 {
            self.write_state(&state)?;
        }
        Ok(removed)
    }

    /// Add an artifact path to a task record.
    pub fn add_artifact(&self, task_id: &str, artifact_path: impl Into<String>) -> Result<bool> {
        let mut state = self.load_state_or_default()?;
        let Some(record) = state.tasks.iter_mut().find(|t| t.id == task_id) else {
            return Ok(false);
        };
        record.artifact_paths.push(artifact_path.into());
        self.write_state(&state)?;
        Ok(true)
    }

    /// Append a log line to a task-specific log file under `base_dir / logs / {task_id}.log`.
    /// The log file path is automatically added to the task's artifact_paths on first write.
    pub fn append_log(&self, task_id: &str, line: impl AsRef<str>) -> Result<()> {
        let task_dir = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("logs");
        fs::create_dir_all(&task_dir)?;

        let log_path = task_dir.join(format!("{}.log", task_id));

        let unix = unix_secs();
        let log_line = format!("[{}] {}\n", unix, line.as_ref());
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?
            .write_all(log_line.as_bytes())?;

        // Ensure the log path is in artifact_paths
        let log_path_str = log_path.to_string_lossy().to_string();
        let mut state = self.load_state_or_default()?;
        if let Some(record) = state.tasks.iter_mut().find(|t| t.id == task_id) {
            if !record.artifact_paths.contains(&log_path_str) {
                record.artifact_paths.push(log_path_str);
                self.write_state(&state)?;
            }
        }
        Ok(())
    }

    /// Read the log file for a task. Returns None if the log file doesn't exist.
    pub fn read_log(&self, task_id: &str) -> Result<Option<String>> {
        let log_path = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("logs")
            .join(format!("{}.log", task_id));
        if !log_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&log_path)?;
        Ok(Some(content))
    }

    /// Return the base directory where task data (logs, artifacts) is stored.
    pub fn base_dir(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    // ── Internals ──

    fn load_state_or_default(&self) -> Result<DurableTaskManagerState> {
        if !self.path.exists() {
            return Ok(DurableTaskManagerState::default());
        }
        let bytes = fs::read(&self.path)?;
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    }

    fn write_state(&self, state: &DurableTaskManagerState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, serde_json::to_string_pretty(state)?)?;
        Ok(())
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_record() {
        let record = DurableTaskRecord::new("task-1", "Build project", "cargo build");
        assert_eq!(record.id, "task-1");
        assert_eq!(record.label, "Build project");
        assert_eq!(record.kind, "cargo build");
        assert_eq!(record.status, DurableTaskStatus::Queued);
        assert!(record.created_at_unix > 0);
        assert!(record.started_at_unix.is_none());
        assert!(record.completed_at_unix.is_none());
    }

    #[test]
    fn test_mark_running() {
        let mut record = DurableTaskRecord::new("t1", "Test", "test");
        record.mark_running();
        assert_eq!(record.status, DurableTaskStatus::Running);
        assert!(record.started_at_unix.is_some());
    }

    #[test]
    fn test_mark_completed() {
        let mut record = DurableTaskRecord::new("t1", "Test", "test");
        record.mark_running();
        record.mark_completed("1 test passed");
        assert_eq!(record.status, DurableTaskStatus::Completed);
        assert!(record.completed_at_unix.is_some());
        assert_eq!(record.result_summary, Some("1 test passed".to_string()));
    }

    #[test]
    fn test_mark_failed() {
        let mut record = DurableTaskRecord::new("t1", "Test", "test");
        record.mark_failed("timeout");
        assert_eq!(record.status, DurableTaskStatus::Failed);
        assert_eq!(record.error_message, Some("timeout".to_string()));
    }

    #[test]
    fn test_mark_canceled() {
        let mut record = DurableTaskRecord::new("t1", "Test", "test");
        record.mark_canceled();
        assert_eq!(record.status, DurableTaskStatus::Canceled);
        assert!(record.completed_at_unix.is_some());
    }

    #[test]
    fn test_manager_create_and_load() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        let record = mgr.create("t1", "Build", "cargo").unwrap();
        assert_eq!(record.label, "Build");

        let loaded = mgr.load("t1").unwrap().unwrap();
        assert_eq!(loaded.id, "t1");
        assert_eq!(loaded.status, DurableTaskStatus::Queued);
        clean(&dir);
    }

    #[test]
    fn test_manager_save_and_update() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        mgr.create("t1", "Build", "cargo").unwrap();

        let updated = mgr
            .update_status("t1", DurableTaskStatus::Running)
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, DurableTaskStatus::Running);
        assert!(updated.started_at_unix.is_some());

        let loaded = mgr.load("t1").unwrap().unwrap();
        assert_eq!(loaded.status, DurableTaskStatus::Running);
        assert!(loaded.started_at_unix.is_some());
        clean(&dir);
    }

    #[test]
    fn test_manager_update_nonexistent() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        let result = mgr
            .update_status("ghost", DurableTaskStatus::Running)
            .unwrap();
        assert!(result.is_none());
        clean(&dir);
    }

    #[test]
    fn test_manager_persistence_survives_restart() {
        let dir = temp_dir();
        {
            let mgr = DurableTaskManager::open_in(&dir).unwrap();
            mgr.create("t1", "Build", "cargo").unwrap();
            mgr.update_status("t1", DurableTaskStatus::Running).unwrap();
        }
        // Reopen — simulates app restart
        {
            let mgr = DurableTaskManager::open_in(&dir).unwrap();
            let loaded = mgr.load("t1").unwrap().unwrap();
            assert_eq!(loaded.status, DurableTaskStatus::Running);
            assert!(loaded.started_at_unix.is_some());
        }
        clean(&dir);
    }

    #[test]
    fn test_manager_list() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        mgr.create("a", "Task A", "type").unwrap();
        mgr.create("b", "Task B", "type").unwrap();

        let all = mgr.load_all().unwrap();
        assert_eq!(all.len(), 2);
        clean(&dir);
    }

    #[test]
    fn test_manager_delete() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        mgr.create("t1", "Delete me", "test").unwrap();
        assert!(mgr.delete("t1").unwrap());
        assert!(mgr.load("t1").unwrap().is_none());
        assert!(mgr.load_all().unwrap().is_empty());
        clean(&dir);
    }

    #[test]
    fn test_manager_delete_nonexistent() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        assert!(!mgr.delete("ghost").unwrap());
        clean(&dir);
    }

    #[test]
    fn test_manager_count_by_status() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        let t1 = mgr.create("a", "A", "type").unwrap();
        mgr.save(&DurableTaskRecord {
            status: DurableTaskStatus::Running,
            started_at_unix: Some(100),
            ..t1
        })
        .unwrap();
        mgr.create("b", "B", "type").unwrap();

        let counts = mgr.count_by_status().unwrap();
        assert_eq!(*counts.get("Running").unwrap_or(&0), 1);
        assert_eq!(*counts.get("Queued").unwrap_or(&0), 1);
        clean(&dir);
    }

    #[test]
    fn test_prune_terminal_tasks() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();

        // One completed, one queued
        let t1 = DurableTaskRecord {
            status: DurableTaskStatus::Completed,
            completed_at_unix: Some(100),
            ..DurableTaskRecord::new("done", "Done", "t")
        };
        let t2 = DurableTaskRecord::new("pending", "Pending", "t");
        mgr.save(&t1).unwrap();
        mgr.save(&t2).unwrap();

        let pruned = mgr.prune_terminal_tasks().unwrap();
        assert_eq!(pruned, 1);

        let remaining = mgr.load_all().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "pending");
        clean(&dir);
    }

    #[test]
    fn test_empty_list_when_no_file() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        assert!(mgr.load_all().unwrap().is_empty());
        clean(&dir);
    }

    #[test]
    fn test_add_artifact_methods() {
        let mut record = DurableTaskRecord::new("t1", "Build", "cargo");
        assert!(!record.has_artifacts());
        assert_eq!(record.artifact_count(), 0);
        record.add_artifact("/tmp/build.log");
        assert!(record.has_artifacts());
        assert_eq!(record.artifact_count(), 1);
        assert_eq!(record.artifact_paths[0], "/tmp/build.log");
    }

    #[test]
    fn test_manager_add_artifact() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        mgr.create("t1", "Build", "cargo").unwrap();

        assert!(mgr.add_artifact("t1", "/tmp/build.log").unwrap());
        let loaded = mgr.load("t1").unwrap().unwrap();
        assert_eq!(loaded.artifact_paths.len(), 1);
        assert_eq!(loaded.artifact_paths[0], "/tmp/build.log");
        clean(&dir);
    }

    #[test]
    fn test_manager_add_artifact_nonexistent() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        assert!(!mgr.add_artifact("ghost", "/tmp/x.log").unwrap());
        clean(&dir);
    }

    #[test]
    fn test_manager_append_and_read_log() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        mgr.create("t1", "Build", "cargo").unwrap();

        mgr.append_log("t1", "starting build").unwrap();
        mgr.append_log("t1", "compilation ok").unwrap();

        // Check log content
        let log = mgr.read_log("t1").unwrap().expect("log should exist");
        assert!(log.contains("starting build"));
        assert!(log.contains("compilation ok"));

        // Check that the log path is tracked as an artifact
        let loaded = mgr.load("t1").unwrap().unwrap();
        assert_eq!(loaded.artifact_paths.len(), 1);
        assert!(loaded.artifact_paths[0].ends_with("t1.log"));
        clean(&dir);
    }

    #[test]
    fn test_manager_read_log_nonexistent() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        assert!(mgr.read_log("ghost").unwrap().is_none());
        clean(&dir);
    }

    #[test]
    fn test_manager_append_log_nonexistent_task() {
        let dir = temp_dir();
        let mgr = DurableTaskManager::open_in(&dir).unwrap();
        // Should still write the log file even if the task record doesn't exist
        mgr.append_log("norecord", "some output").unwrap();
        let log = mgr.read_log("norecord").unwrap().expect("log should exist");
        assert!(log.contains("some output"));
        clean(&dir);
    }

    #[test]
    fn test_is_terminal() {
        assert!(!DurableTaskStatus::Queued.is_terminal());
        assert!(!DurableTaskStatus::Running.is_terminal());
        assert!(DurableTaskStatus::Completed.is_terminal());
        assert!(DurableTaskStatus::Failed.is_terminal());
        assert!(DurableTaskStatus::Canceled.is_terminal());
    }

    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "deepseek-durable-task-test-{}-{}-{}",
            std::process::id(),
            unix_secs(),
            id,
        ))
    }

    fn clean(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }
}
