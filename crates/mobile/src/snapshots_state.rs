use deepseek_mobile_core::{AgentEvent, WorkspaceSnapshotRecord};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SnapshotsUiState {
    pub snapshots: Vec<WorkspaceSnapshotRecord>,
    pub last_error: Option<String>,
    /// Set when the user has selected a snapshot for restore but not yet confirmed
    pub pending_restore_snapshot_id: Option<String>,
    pub restore_in_progress: bool,
    pub last_restore_report: Option<String>,
}

impl SnapshotsUiState {
    pub fn apply_agent_event(&mut self, event: &AgentEvent) {
        let AgentEvent::ToolCallFinished(result) = event else {
            return;
        };
        let Some(metadata) = result.metadata.as_ref() else {
            return;
        };

        if let Some(pre_snapshot) = metadata
            .get("pre_snapshot")
            .and_then(|value| serde_json::from_value::<WorkspaceSnapshotRecord>(value.clone()).ok())
        {
            self.upsert(pre_snapshot);
        }

        match result.name.as_str() {
            "snapshot_create" => {
                match serde_json::from_value::<WorkspaceSnapshotRecord>(metadata.clone()) {
                    Ok(snapshot) => {
                        self.upsert(snapshot);
                        self.last_error = None;
                    }
                    Err(error) => {
                        self.last_error = Some(format!(
                            "Failed to read created snapshot metadata: {}",
                            error
                        ));
                    }
                }
            }
            "snapshot_list" => {
                match serde_json::from_value::<Vec<WorkspaceSnapshotRecord>>(metadata.clone()) {
                    Ok(mut snapshots) => {
                        snapshots.sort_by(|left, right| right.created_unix.cmp(&left.created_unix));
                        self.snapshots = snapshots;
                        self.last_error = None;
                    }
                    Err(error) => {
                        self.last_error =
                            Some(format!("Failed to read snapshot list metadata: {}", error));
                    }
                }
            }
            "snapshot_restore" => {
                self.pending_restore_snapshot_id = None;
                self.restore_in_progress = false;
                if result.success {
                    self.last_restore_report = Some(format!(
                        "Restore completed: {} files restored",
                        metadata
                            .get("restored_files")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0)
                    ));
                } else {
                    self.last_error = Some(format!("Restore failed: {}", result.output));
                }
            }
            _ => {}
        }
    }

    /// Start the restore flow for a snapshot — shows the confirmation step.
    pub fn request_restore(&mut self, snapshot_id: &str) {
        self.pending_restore_snapshot_id = Some(snapshot_id.to_string());
        self.last_error = None;
    }

    /// Cancel a pending restore request.
    pub fn cancel_restore(&mut self) {
        self.pending_restore_snapshot_id = None;
    }

    /// Confirm and mark restore as in progress.
    pub fn confirm_restore(&mut self) {
        self.restore_in_progress = true;
    }

    /// The currently selected snapshot for restore, if any and pending confirmation.
    pub fn pending_restore_snapshot(&self) -> Option<&WorkspaceSnapshotRecord> {
        self.pending_restore_snapshot_id
            .as_ref()
            .and_then(|id| self.snapshots.iter().find(|s| s.id == *id))
    }

    fn upsert(&mut self, snapshot: WorkspaceSnapshotRecord) {
        self.snapshots.retain(|item| item.id != snapshot.id);
        self.snapshots.push(snapshot);
        self.snapshots
            .sort_by(|left, right| right.created_unix.cmp(&left.created_unix));
    }

    pub fn latest(&self) -> Option<&WorkspaceSnapshotRecord> {
        self.snapshots.first()
    }
}

#[cfg(test)]
mod tests {
    use super::SnapshotsUiState;
    use deepseek_mobile_core::{AgentEvent, ToolResultEvent};

    #[test]
    fn pre_tool_snapshot_metadata_is_tracked() {
        let mut state = SnapshotsUiState::default();
        state.apply_agent_event(&AgentEvent::ToolCallFinished(ToolResultEvent {
            id: "tool-1".to_string(),
            name: "write_file".to_string(),
            success: true,
            output: "ok".to_string(),
            metadata: Some(serde_json::json!({
                "pre_snapshot": {
                    "schema_version": 1,
                    "id": "snapshot-1",
                    "workspace_id": "w1",
                    "workspace_name": "Workspace",
                    "workspace_root": ".",
                    "reason": "pre-tool",
                    "created_unix": 10,
                    "file_count": 2,
                    "total_bytes": 42,
                    "files": []
                }
            })),
        }));

        assert_eq!(state.snapshots.len(), 1);
        assert_eq!(state.latest().unwrap().id, "snapshot-1");
    }

    #[test]
    fn restore_flow_states_are_tracked() {
        let mut state = SnapshotsUiState::default();
        let snapshot = deepseek_mobile_core::WorkspaceSnapshotRecord {
            schema_version: 1,
            id: "snap-1".to_string(),
            workspace_id: "w1".to_string(),
            workspace_name: "Workspace".to_string(),
            workspace_root: ".".into(),
            reason: "test".to_string(),
            created_unix: 100,
            file_count: 5,
            total_bytes: 500,
            files: vec![],
        };
        state.snapshots.push(snapshot);

        state.request_restore("snap-1");
        assert_eq!(
            state.pending_restore_snapshot_id,
            Some("snap-1".to_string())
        );
        assert!(state.pending_restore_snapshot().is_some());

        state.cancel_restore();
        assert!(state.pending_restore_snapshot_id.is_none());

        state.request_restore("snap-1");
        state.confirm_restore();
        assert!(state.restore_in_progress);
    }
}
