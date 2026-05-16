use deepseek_mobile_core::{AgentEvent, WorkspaceSnapshotRecord};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SnapshotsUiState {
    pub snapshots: Vec<WorkspaceSnapshotRecord>,
    pub last_error: Option<String>,
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
            _ => {}
        }
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
}
