use deepseek_mobile_core::{
    DurableTaskManager, DurableTaskRecord, DurableTaskStatus, PcGatewayClient, PcGatewayResponse,
    PcRunningTaskEvent, PcRunningTaskInfo,
};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mobile_runtime_config::default_data_dir;

#[derive(Clone, Debug)]
pub struct TasksUiState {
    pub tasks: Vec<DurableTaskRecord>,
    pub pc_running_tasks: Vec<PcRunningTaskInfo>,
    pub last_error: Option<String>,
    pub pc_last_error: Option<String>,
    pub pc_last_synced_at_unix: Option<u64>,
    pub filter_status: Option<DurableTaskStatus>,
}

impl Default for TasksUiState {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            pc_running_tasks: Vec::new(),
            last_error: None,
            pc_last_error: None,
            pc_last_synced_at_unix: None,
            filter_status: None,
        }
    }
}

impl TasksUiState {
    /// Load all tasks from the durable task manager.
    pub fn refresh(&mut self) {
        let mgr = match DurableTaskManager::open(default_data_dir().join("tasks.json")) {
            Ok(m) => m,
            Err(e) => {
                self.last_error = Some(format!("Failed to open task manager: {}", e));
                return;
            }
        };
        match mgr.load_all() {
            Ok(mut tasks) => {
                tasks.sort_by(|a, b| b.created_at_unix.cmp(&a.created_at_unix));
                if let Some(filter) = &self.filter_status {
                    let filter = filter.clone();
                    tasks.retain(|t| t.status == filter);
                }
                self.tasks = tasks;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to load tasks: {}", e));
            }
        }
    }

    /// Refresh currently running tasks from the active PC host.
    pub async fn refresh_pc_running_tasks(&mut self, client: &PcGatewayClient) {
        match client.list_tasks().await {
            Ok(PcGatewayResponse::TaskList(tasks)) => self.apply_pc_running_tasks(tasks),
            Ok(other) => {
                self.pc_last_error = Some(format!("Unexpected PC task response: {:?}", other));
            }
            Err(error) => {
                self.pc_last_error = Some(format!("Failed to sync PC tasks: {}", error));
            }
        }
    }

    pub fn apply_pc_running_tasks(&mut self, mut tasks: Vec<PcRunningTaskInfo>) {
        tasks.sort_by(|left, right| {
            right
                .started_at_unix
                .cmp(&left.started_at_unix)
                .then(left.label.cmp(&right.label))
                .then(left.id.cmp(&right.id))
        });
        self.pc_running_tasks = tasks;
        self.pc_last_error = None;
        self.pc_last_synced_at_unix = Some(current_unix_time());
    }

    /// Apply a single live PC task event from the SSE stream.
    pub fn apply_pc_event(&mut self, event: &PcRunningTaskEvent) {
        match event {
            PcRunningTaskEvent::TaskStarted(info) => {
                // Insert or replace the task in the running list
                self.pc_running_tasks.retain(|t| t.id != info.id);
                self.pc_running_tasks.push(info.clone());
                self.pc_running_tasks
                    .sort_by(|a, b| b.started_at_unix.cmp(&a.started_at_unix));
            }
            PcRunningTaskEvent::TaskCompleted { task_id, .. }
            | PcRunningTaskEvent::TaskFailed { task_id, .. }
            | PcRunningTaskEvent::TaskStopped { task_id } => {
                self.pc_running_tasks.retain(|t| t.id != *task_id);
            }
        }
        self.pc_last_synced_at_unix = Some(current_unix_time());
    }

    pub fn clear_pc_running_tasks(&mut self) {
        self.pc_running_tasks.clear();
        self.pc_last_error = None;
        self.pc_last_synced_at_unix = None;
    }

    pub fn active_count(&self) -> usize {
        let pc_ids: HashSet<&str> = self
            .pc_running_tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect();
        let local_active = self
            .tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.status,
                    DurableTaskStatus::Queued | DurableTaskStatus::Running
                ) && !pc_ids.contains(task.id.as_str())
            })
            .count();
        local_active + self.pc_running_tasks.len()
    }

    /// Cancel a queued or running task.
    pub fn cancel_task(&mut self, task_id: &str) {
        let mgr = match DurableTaskManager::open(default_data_dir().join("tasks.json")) {
            Ok(m) => m,
            Err(e) => {
                self.last_error = Some(format!("Failed to open task manager: {}", e));
                return;
            }
        };
        match mgr.update_status(task_id, DurableTaskStatus::Canceled) {
            Ok(Some(_)) => {
                self.last_error = None;
                self.refresh();
            }
            Ok(None) => {
                self.last_error = Some("Task not found.".to_string());
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to cancel task: {}", e));
            }
        }
    }

    /// Prune all completed/failed/canceled tasks.
    pub fn prune_terminal(&mut self) {
        let mgr = match DurableTaskManager::open(default_data_dir().join("tasks.json")) {
            Ok(m) => m,
            Err(e) => {
                self.last_error = Some(format!("Failed to open task manager: {}", e));
                return;
            }
        };
        match mgr.prune_terminal_tasks() {
            Ok(removed) => {
                self.last_error = None;
                if removed > 0 {
                    self.refresh();
                }
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to prune tasks: {}", e));
            }
        }
    }

    /// Set filter status to show only tasks of a given status.
    pub fn set_filter(&mut self, status: Option<DurableTaskStatus>) {
        self.filter_status = status;
        self.refresh();
    }
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::TasksUiState;
    use deepseek_mobile_core::{
        DurableTaskRecord, DurableTaskStatus, PcRunningTaskEvent, PcRunningTaskInfo,
    };

    #[test]
    fn pc_running_tasks_are_sorted_newest_first() {
        let mut state = TasksUiState::default();
        state.apply_pc_running_tasks(vec![
            pc_task("old", "Old", 10),
            pc_task("new", "New", 20),
        ]);

        assert_eq!(state.pc_running_tasks[0].id, "new");
        assert_eq!(state.pc_running_tasks[1].id, "old");
        assert!(state.pc_last_synced_at_unix.is_some());
        assert!(state.pc_last_error.is_none());
    }

    #[test]
    fn active_count_reconciles_local_and_pc_tasks_by_id() {
        let mut state = TasksUiState::default();
        state.tasks = vec![
            local_task("same", DurableTaskStatus::Running),
            local_task("queued", DurableTaskStatus::Queued),
            local_task("done", DurableTaskStatus::Completed),
        ];
        state.apply_pc_running_tasks(vec![pc_task("same", "Same", 20), pc_task("pc", "PC", 30)]);

        assert_eq!(state.active_count(), 3);
    }

    #[test]
    fn clear_pc_running_tasks_resets_sync_state() {
        let mut state = TasksUiState::default();
        state.apply_pc_running_tasks(vec![pc_task("pc", "PC", 30)]);
        state.clear_pc_running_tasks();

        assert!(state.pc_running_tasks.is_empty());
        assert!(state.pc_last_error.is_none());
        assert!(state.pc_last_synced_at_unix.is_none());
    }

    #[test]
    fn apply_pc_event_task_started_adds_to_list() {
        let mut state = TasksUiState::default();
        state.apply_pc_event(&PcRunningTaskEvent::TaskStarted(pc_task("t1", "Task 1", 100)));
        assert_eq!(state.pc_running_tasks.len(), 1);
        assert_eq!(state.pc_running_tasks[0].id, "t1");
        assert!(state.pc_last_synced_at_unix.is_some());
    }

    #[test]
    fn apply_pc_event_task_started_replaces_existing() {
        let mut state = TasksUiState::default();
        state.apply_pc_event(&PcRunningTaskEvent::TaskStarted(pc_task("t1", "Old", 100)));
        state.apply_pc_event(&PcRunningTaskEvent::TaskStarted(pc_task("t1", "New", 200)));
        assert_eq!(state.pc_running_tasks.len(), 1);
        assert_eq!(state.pc_running_tasks[0].label, "New");
    }

    #[test]
    fn apply_pc_event_task_completed_removes() {
        let mut state = TasksUiState::default();
        state.apply_pc_running_tasks(vec![pc_task("t1", "T1", 100), pc_task("t2", "T2", 200)]);
        state.apply_pc_event(&PcRunningTaskEvent::TaskCompleted {
            task_id: "t1".to_string(),
            exit_code: Some(0),
        });
        assert_eq!(state.pc_running_tasks.len(), 1);
        assert_eq!(state.pc_running_tasks[0].id, "t2");
    }

    #[test]
    fn apply_pc_event_task_failed_removes() {
        let mut state = TasksUiState::default();
        state.apply_pc_running_tasks(vec![pc_task("t1", "T1", 100)]);
        state.apply_pc_event(&PcRunningTaskEvent::TaskFailed {
            task_id: "t1".to_string(),
            error: "boom".to_string(),
        });
        assert!(state.pc_running_tasks.is_empty());
    }

    #[test]
    fn apply_pc_event_task_stopped_removes() {
        let mut state = TasksUiState::default();
        state.apply_pc_running_tasks(vec![pc_task("t1", "T1", 100)]);
        state.apply_pc_event(&PcRunningTaskEvent::TaskStopped {
            task_id: "t1".to_string(),
        });
        assert!(state.pc_running_tasks.is_empty());
    }

    fn pc_task(id: &str, label: &str, started_at_unix: u64) -> PcRunningTaskInfo {
        PcRunningTaskInfo {
            id: id.to_string(),
            label: label.to_string(),
            kind: "test".to_string(),
            started_at_unix,
        }
    }

    fn local_task(id: &str, status: DurableTaskStatus) -> DurableTaskRecord {
        let mut task = DurableTaskRecord::new(id, id, "test");
        task.status = status;
        task
    }
}
