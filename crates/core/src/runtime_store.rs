//! Durable runtime store for mobile turns and approvals.
//!
//! The Android UI can close/reopen while a turn is waiting for approval. This
//! store persists turns, pending approval records and emitted events as compact
//! JSON files under the configured runtime root.

use crate::events::AgentEvent;
use crate::tool_loop::PendingToolCallApproval;
use crate::turn::{TokenUsage, TurnContext, TurnStatus};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeTurnStatus {
    Queued,
    Running,
    WaitingForApproval,
    Completed,
    Failed,
    Cancelled,
}

impl From<&TurnStatus> for RuntimeTurnStatus {
    fn from(status: &TurnStatus) -> Self {
        match status {
            TurnStatus::Queued => RuntimeTurnStatus::Queued,
            TurnStatus::Running => RuntimeTurnStatus::Running,
            TurnStatus::WaitingForApproval => RuntimeTurnStatus::WaitingForApproval,
            TurnStatus::Completed => RuntimeTurnStatus::Completed,
            TurnStatus::Failed => RuntimeTurnStatus::Failed,
            TurnStatus::Cancelled => RuntimeTurnStatus::Cancelled,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ThreadRecord {
    pub id: String,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub turn_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TurnRecord {
    pub id: String,
    pub thread_id: String,
    pub status: RuntimeTurnStatus,
    pub usage_input_tokens: u64,
    pub usage_output_tokens: u64,
    pub usage_reasoning_tokens: u64,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub error: Option<String>,
}

impl TurnRecord {
    pub fn from_turn(thread_id: impl Into<String>, turn: &TurnContext) -> Self {
        Self {
            id: turn.id.clone(),
            thread_id: thread_id.into(),
            status: RuntimeTurnStatus::from(&turn.status),
            usage_input_tokens: turn.usage.input_tokens,
            usage_output_tokens: turn.usage.output_tokens,
            usage_reasoning_tokens: turn.usage.reasoning_tokens,
            created_at_unix: turn.created_at_unix,
            updated_at_unix: turn.updated_at_unix,
            error: turn.error.clone(),
        }
    }

    pub fn usage(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.usage_input_tokens,
            output_tokens: self.usage_output_tokens,
            reasoning_tokens: self.usage_reasoning_tokens,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TurnItemKind {
    Event,
    ToolCall,
    ToolResult,
    Approval,
    Message,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnItemLifecycleStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TurnItemRecord {
    pub id: String,
    pub turn_id: String,
    pub kind: TurnItemKind,
    pub status: TurnItemLifecycleStatus,
    pub payload: serde_json::Value,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RuntimeEventRecord {
    pub id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub event: AgentEvent,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingApprovalRecord {
    pub approval_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub pending: PendingToolCallApproval,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApprovalDecisionRecord {
    pub approval_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub decision: String,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug)]
pub struct RuntimeThreadStore {
    root: PathBuf,
}

impl RuntimeThreadStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("threads"))?;
        fs::create_dir_all(root.join("turns"))?;
        fs::create_dir_all(root.join("events"))?;
        fs::create_dir_all(root.join("pending_approvals"))?;
        fs::create_dir_all(root.join("decisions"))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn save_thread(&self, record: &ThreadRecord) -> Result<()> {
        write_json(self.thread_path(&record.id), record)
    }

    pub fn load_thread(&self, thread_id: &str) -> Result<ThreadRecord> {
        read_json(self.thread_path(thread_id))
    }

    pub fn ensure_thread(&self, thread_id: impl Into<String>) -> Result<ThreadRecord> {
        let thread_id = thread_id.into();
        match self.load_thread(&thread_id) {
            Ok(record) => Ok(record),
            Err(_) => {
                let now = current_unix_time();
                let record = ThreadRecord {
                    id: thread_id,
                    created_at_unix: now,
                    updated_at_unix: now,
                    turn_ids: Vec::new(),
                };
                self.save_thread(&record)?;
                Ok(record)
            }
        }
    }

    pub fn save_turn(&self, record: &TurnRecord) -> Result<()> {
        let mut thread = self.ensure_thread(record.thread_id.clone())?;
        if !thread.turn_ids.contains(&record.id) {
            thread.turn_ids.push(record.id.clone());
        }
        thread.updated_at_unix = current_unix_time();
        self.save_thread(&thread)?;
        write_json(self.turn_path(&record.id), record)
    }

    pub fn load_turn(&self, turn_id: &str) -> Result<TurnRecord> {
        read_json(self.turn_path(turn_id))
    }

    pub fn save_event(&self, thread_id: impl Into<String>, turn_id: impl Into<String>, event: &AgentEvent) -> Result<RuntimeEventRecord> {
        let record = RuntimeEventRecord {
            id: format!("event-{}", current_unix_time_nanos()),
            thread_id: thread_id.into(),
            turn_id: turn_id.into(),
            event: event.clone(),
            created_at_unix: current_unix_time(),
        };
        write_json(self.event_path(&record.id), &record)?;
        Ok(record)
    }

    pub fn save_pending_approval(
        &self,
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        pending: PendingToolCallApproval,
    ) -> Result<PendingApprovalRecord> {
        let record = PendingApprovalRecord {
            approval_id: pending.approval.id.clone(),
            thread_id: thread_id.into(),
            turn_id: turn_id.into(),
            pending,
            created_at_unix: current_unix_time(),
        };
        write_json(self.pending_path(&record.approval_id), &record)?;
        Ok(record)
    }

    pub fn load_pending_approval(&self, approval_id: &str) -> Result<PendingApprovalRecord> {
        read_json(self.pending_path(approval_id))
    }

    pub fn remove_pending_approval(&self, approval_id: &str) -> Result<()> {
        let path = self.pending_path(approval_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn list_pending_approvals(&self) -> Result<Vec<PendingApprovalRecord>> {
        read_json_dir(self.root.join("pending_approvals"))
    }

    pub fn list_pending_approvals_for_thread(&self, thread_id: &str) -> Result<Vec<PendingApprovalRecord>> {
        Ok(self
            .list_pending_approvals()?
            .into_iter()
            .filter(|record| record.thread_id == thread_id)
            .collect())
    }

    pub fn save_decision(&self, record: &ApprovalDecisionRecord) -> Result<()> {
        write_json(self.decision_path(&record.approval_id), record)
    }

    fn thread_path(&self, id: &str) -> PathBuf {
        self.root.join("threads").join(format!("{}.json", safe_file_id(id)))
    }

    fn turn_path(&self, id: &str) -> PathBuf {
        self.root.join("turns").join(format!("{}.json", safe_file_id(id)))
    }

    fn event_path(&self, id: &str) -> PathBuf {
        self.root.join("events").join(format!("{}.json", safe_file_id(id)))
    }

    fn pending_path(&self, id: &str) -> PathBuf {
        self.root.join("pending_approvals").join(format!("{}.json", safe_file_id(id)))
    }

    fn decision_path(&self, id: &str) -> PathBuf {
        self.root.join("decisions").join(format!("{}.json", safe_file_id(id)))
    }
}

fn write_json(path: PathBuf, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let bytes = fs::read(&path).map_err(|error| anyhow!("failed to read {}: {}", path.display(), error))?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_json_dir<T: for<'de> Deserialize<'de>>(dir: PathBuf) -> Result<Vec<T>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(read_json(entry.path())?);
        }
    }
    Ok(out)
}

fn safe_file_id(id: &str) -> String {
    id.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' })
        .collect()
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn current_unix_time_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{RuntimeThreadStore, TurnRecord};
    use crate::turn::TurnContext;
    use std::fs;

    fn unique_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-runtime-store-{}-{}", name, nanos))
    }

    #[test]
    fn saves_and_loads_turn_record() {
        let root = unique_dir("turn");
        let store = RuntimeThreadStore::open(&root).expect("open store");
        let turn = TurnContext::new(10);
        let record = TurnRecord::from_turn("thread-1", &turn);
        store.save_turn(&record).expect("save turn");
        let loaded = store.load_turn(&record.id).expect("load turn");
        assert_eq!(loaded.id, record.id);
        let _ = fs::remove_dir_all(root);
    }
}