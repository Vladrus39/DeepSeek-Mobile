use deepseek_mobile_core::{DurableTaskManager, DurableTaskRecord, DurableTaskStatus};

use crate::mobile_runtime_config::default_data_dir;

#[derive(Clone, Debug)]
pub struct TasksUiState {
    pub tasks: Vec<DurableTaskRecord>,
    pub last_error: Option<String>,
    pub filter_status: Option<DurableTaskStatus>,
}

impl Default for TasksUiState {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            last_error: None,
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
