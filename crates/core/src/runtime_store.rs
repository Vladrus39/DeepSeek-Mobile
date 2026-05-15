//! Durable runtime store for mobile threads, turns, items and events.
//!
//! Android may suspend or kill the app at any time. The core therefore needs a
//! small append-friendly store so the UI can replay the latest timeline after a
//! restart. This is inspired by the original TUI runtime thread store, but kept
//! dependency-light for mobile builds.

use crate::approval::{ApprovalRisk, ReviewDecision, ToolCategory};
use crate::approval_session::ApprovalSessionGrant;
use crate::events::AgentEvent;
use crate::tool_loop::PendingToolCallApproval;
use crate::turn::{TurnContext, TurnStatus};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CURRENT_RUNTIME_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeTurnStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Interrupted,
    Cancelled,
}

impl From<&TurnStatus> for RuntimeTurnStatus {
    fn from(status: &TurnStatus) -> Self {
        match status {
            TurnStatus::Queued => RuntimeTurnStatus::Queued,
            TurnStatus::Running | TurnStatus::WaitingForApproval => RuntimeTurnStatus::InProgress,
            TurnStatus::Completed => RuntimeTurnStatus::Completed,
            TurnStatus::Failed => RuntimeTurnStatus::Failed,
            TurnStatus::Cancelled => RuntimeTurnStatus::Cancelled,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnItemKind {
    UserMessage,
    AgentMessage,
    AgentReasoning,
    ToolCall,
    FileChange,
    CommandExecution,
    Approval,
    ContextCompression,
    Status,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnItemLifecycleStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Interrupted,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadRecord {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub model: String,
    pub workspace: PathBuf,
    pub archived: bool,
    pub latest_turn_id: Option<String>,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl ThreadRecord {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        model: impl Into<String>,
        workspace: impl Into<PathBuf>,
    ) -> Self {
        let now = current_unix_time();
        Self {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            id: id.into(),
            title: title.into(),
            model: model.into(),
            workspace: workspace.into(),
            archived: false,
            latest_turn_id: None,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at_unix = current_unix_time();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnRecord {
    pub schema_version: u32,
    pub id: String,
    pub thread_id: String,
    pub status: RuntimeTurnStatus,
    pub input_summary: String,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub usage_input_tokens: u64,
    pub usage_output_tokens: u64,
    pub usage_reasoning_tokens: u64,
    pub error: Option<String>,
    pub item_ids: Vec<String>,
}

impl TurnRecord {
    pub fn from_context(thread_id: impl Into<String>, input_summary: impl Into<String>, turn: &TurnContext) -> Self {
        Self {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            id: turn.id.clone(),
            thread_id: thread_id.into(),
            status: RuntimeTurnStatus::from(&turn.status),
            input_summary: input_summary.into(),
            created_at_unix: turn.created_at_unix,
            updated_at_unix: turn.updated_at_unix,
            usage_input_tokens: turn.usage.input_tokens,
            usage_output_tokens: turn.usage.output_tokens,
            usage_reasoning_tokens: turn.usage.reasoning_tokens,
            error: turn.error.clone(),
            item_ids: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TurnItemRecord {
    pub schema_version: u32,
    pub id: String,
    pub turn_id: String,
    pub kind: TurnItemKind,
    pub status: TurnItemLifecycleStatus,
    pub summary: String,
    pub detail: Option<String>,
    pub metadata: Option<Value>,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl TurnItemRecord {
    pub fn new(
        id: impl Into<String>,
        turn_id: impl Into<String>,
        kind: TurnItemKind,
        status: TurnItemLifecycleStatus,
        summary: impl Into<String>,
    ) -> Self {
        let now = current_unix_time();
        Self {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            id: id.into(),
            turn_id: turn_id.into(),
            kind,
            status,
            summary: summary.into(),
            detail: None,
            metadata: None,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self.updated_at_unix = current_unix_time();
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self.updated_at_unix = current_unix_time();
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RuntimeEventRecord {
    pub schema_version: u32,
    pub seq: u64,
    pub timestamp_unix: u64,
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub event: AgentEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingApprovalRecord {
    pub schema_version: u32,
    pub thread_id: String,
    pub turn_id: String,
    pub pending: PendingToolCallApproval,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl PendingApprovalRecord {
    pub fn new(
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        pending: PendingToolCallApproval,
    ) -> Self {
        let now = current_unix_time();
        Self {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            thread_id: thread_id.into(),
            turn_id: turn_id.into(),
            pending,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    pub fn approval_id(&self) -> &str {
        &self.pending.approval.id
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApprovalDecisionRecord {
    pub schema_version: u32,
    pub seq: u64,
    pub timestamp_unix: u64,
    pub thread_id: String,
    pub turn_id: String,
    pub approval_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub category: ToolCategory,
    pub risk: ApprovalRisk,
    pub decision: ReviewDecision,
    pub session_grant: Option<ApprovalSessionGrant>,
    pub arguments_preview: Value,
}

impl ApprovalDecisionRecord {
    pub fn new(
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        pending: &PendingToolCallApproval,
        decision: ReviewDecision,
        session_grant: Option<ApprovalSessionGrant>,
    ) -> Self {
        Self {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            seq: 0,
            timestamp_unix: current_unix_time(),
            thread_id: thread_id.into(),
            turn_id: turn_id.into(),
            approval_id: pending.approval.id.clone(),
            tool_call_id: pending.call.id.clone(),
            tool_name: pending.call.name.clone(),
            category: pending.approval.category.clone(),
            risk: pending.approval.risk.clone(),
            decision,
            session_grant,
            arguments_preview: crate::approval_card::sanitize_value_for_preview(&pending.call.arguments),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RuntimeStoreState {
    schema_version: u32,
    next_seq: u64,
}

impl Default for RuntimeStoreState {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            next_seq: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeThreadStore {
    root: PathBuf,
    threads_dir: PathBuf,
    turns_dir: PathBuf,
    items_dir: PathBuf,
    events_dir: PathBuf,
    pending_approvals_dir: PathBuf,
    approval_decisions_dir: PathBuf,
    state_path: PathBuf,
}

impl RuntimeThreadStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let threads_dir = root.join("threads");
        let turns_dir = root.join("turns");
        let items_dir = root.join("items");
        let events_dir = root.join("events");
        let pending_approvals_dir = root.join("pending_approvals");
        let approval_decisions_dir = root.join("approval_decisions");
        fs::create_dir_all(&threads_dir).with_context(|| format!("create {}", threads_dir.display()))?;
        fs::create_dir_all(&turns_dir).with_context(|| format!("create {}", turns_dir.display()))?;
        fs::create_dir_all(&items_dir).with_context(|| format!("create {}", items_dir.display()))?;
        fs::create_dir_all(&events_dir).with_context(|| format!("create {}", events_dir.display()))?;
        fs::create_dir_all(&pending_approvals_dir)
            .with_context(|| format!("create {}", pending_approvals_dir.display()))?;
        fs::create_dir_all(&approval_decisions_dir)
            .with_context(|| format!("create {}", approval_decisions_dir.display()))?;

        let state_path = root.join("state.json");
        if !state_path.exists() {
            write_json_atomic(&state_path, &RuntimeStoreState::default())?;
        }

        Ok(Self {
            root,
            threads_dir,
            turns_dir,
            items_dir,
            events_dir,
            pending_approvals_dir,
            approval_decisions_dir,
            state_path,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn save_thread(&self, thread: &ThreadRecord) -> Result<()> {
        ensure_supported_schema(thread.schema_version, "thread")?;
        write_json_atomic(&self.thread_path(&thread.id), thread)
    }

    pub fn load_thread(&self, thread_id: &str) -> Result<ThreadRecord> {
        let record: ThreadRecord = read_json(&self.thread_path(thread_id))?;
        ensure_supported_schema(record.schema_version, "thread")?;
        Ok(record)
    }

    pub fn save_turn(&self, turn: &TurnRecord) -> Result<()> {
        ensure_supported_schema(turn.schema_version, "turn")?;
        write_json_atomic(&self.turn_path(&turn.id), turn)
    }

    pub fn load_turn(&self, turn_id: &str) -> Result<TurnRecord> {
        let record: TurnRecord = read_json(&self.turn_path(turn_id))?;
        ensure_supported_schema(record.schema_version, "turn")?;
        Ok(record)
    }

    pub fn save_item(&self, item: &TurnItemRecord) -> Result<()> {
        ensure_supported_schema(item.schema_version, "item")?;
        write_json_atomic(&self.item_path(&item.id), item)
    }

    pub fn load_item(&self, item_id: &str) -> Result<TurnItemRecord> {
        let record: TurnItemRecord = read_json(&self.item_path(item_id))?;
        ensure_supported_schema(record.schema_version, "item")?;
        Ok(record)
    }

    pub fn save_pending_approval(
        &self,
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        pending: &PendingToolCallApproval,
    ) -> Result<PendingApprovalRecord> {
        let record = PendingApprovalRecord::new(thread_id, turn_id, pending.clone());
        write_json_atomic(&self.pending_approval_path(record.approval_id()), &record)?;
        Ok(record)
    }

    pub fn load_pending_approval(&self, approval_id: &str) -> Result<PendingApprovalRecord> {
        let record: PendingApprovalRecord = read_json(&self.pending_approval_path(approval_id))?;
        ensure_supported_schema(record.schema_version, "pending approval")?;
        Ok(record)
    }

    pub fn delete_pending_approval(&self, approval_id: &str) -> Result<()> {
        let path = self.pending_approval_path(approval_id);
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
        }
        Ok(())
    }

    pub fn load_pending_approvals_for_turn(&self, turn_id: &str) -> Result<Vec<PendingApprovalRecord>> {
        let mut records = self.load_all_pending_approvals()?;
        records.retain(|record| record.turn_id == turn_id);
        records.sort_by_key(|record| record.created_at_unix);
        Ok(records)
    }

    pub fn load_pending_approvals_for_thread(&self, thread_id: &str) -> Result<Vec<PendingApprovalRecord>> {
        let mut records = self.load_all_pending_approvals()?;
        records.retain(|record| record.thread_id == thread_id);
        records.sort_by_key(|record| record.created_at_unix);
        Ok(records)
    }

    pub fn load_all_pending_approvals(&self) -> Result<Vec<PendingApprovalRecord>> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.pending_approvals_dir)
            .with_context(|| format!("read {}", self.pending_approvals_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let record: PendingApprovalRecord = read_json(&path)?;
            ensure_supported_schema(record.schema_version, "pending approval")?;
            records.push(record);
        }
        records.sort_by_key(|record| record.created_at_unix);
        Ok(records)
    }

    pub fn append_approval_decision(
        &self,
        mut record: ApprovalDecisionRecord,
    ) -> Result<ApprovalDecisionRecord> {
        ensure_supported_schema(record.schema_version, "approval decision")?;
        let mut state = self.load_state()?;
        record.seq = state.next_seq;
        record.timestamp_unix = current_unix_time();
        state.next_seq = state.next_seq.saturating_add(1);
        self.save_state(&state)?;

        let path = self.approval_decisions_path(&record.thread_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        let line = serde_json::to_string(&record)?;
        writeln!(file, "{}", line)?;
        Ok(record)
    }

    pub fn load_approval_decisions(&self, thread_id: &str) -> Result<Vec<ApprovalDecisionRecord>> {
        let path = self.approval_decisions_path(thread_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut decisions = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ApprovalDecisionRecord = serde_json::from_str(&line)
                .with_context(|| format!("parse approval decision line in {}", path.display()))?;
            ensure_supported_schema(record.schema_version, "approval decision")?;
            decisions.push(record);
        }
        decisions.sort_by_key(|record| record.seq);
        Ok(decisions)
    }

    pub fn load_approval_decisions_for_turn(&self, turn_id: &str) -> Result<Vec<ApprovalDecisionRecord>> {
        let mut decisions = Vec::new();
        for entry in fs::read_dir(&self.approval_decisions_dir)
            .with_context(|| format!("read {}", self.approval_decisions_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                continue;
            }
            let file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let record: ApprovalDecisionRecord = serde_json::from_str(&line)
                    .with_context(|| format!("parse approval decision line in {}", path.display()))?;
                ensure_supported_schema(record.schema_version, "approval decision")?;
                if record.turn_id == turn_id {
                    decisions.push(record);
                }
            }
        }
        decisions.sort_by_key(|record| record.seq);
        Ok(decisions)
    }

    pub fn append_event(
        &self,
        thread_id: impl Into<String>,
        turn_id: Option<String>,
        event: AgentEvent,
    ) -> Result<RuntimeEventRecord> {
        let thread_id = thread_id.into();
        let mut state = self.load_state()?;
        let record = RuntimeEventRecord {
            schema_version: CURRENT_RUNTIME_SCHEMA_VERSION,
            seq: state.next_seq,
            timestamp_unix: current_unix_time(),
            thread_id: thread_id.clone(),
            turn_id,
            event,
        };
        state.next_seq = state.next_seq.saturating_add(1);
        self.save_state(&state)?;

        let path = self.events_path(&thread_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        let line = serde_json::to_string(&record)?;
        writeln!(file, "{}", line)?;
        Ok(record)
    }

    pub fn load_events(&self, thread_id: &str) -> Result<Vec<RuntimeEventRecord>> {
        let path = self.events_path(thread_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: RuntimeEventRecord = serde_json::from_str(&line)
                .with_context(|| format!("parse event line in {}", path.display()))?;
            ensure_supported_schema(record.schema_version, "runtime event")?;
            events.push(record);
        }
        Ok(events)
    }

    fn thread_path(&self, thread_id: &str) -> PathBuf {
        self.threads_dir.join(format!("{}.json", sanitize_id(thread_id)))
    }

    fn turn_path(&self, turn_id: &str) -> PathBuf {
        self.turns_dir.join(format!("{}.json", sanitize_id(turn_id)))
    }

    fn item_path(&self, item_id: &str) -> PathBuf {
        self.items_dir.join(format!("{}.json", sanitize_id(item_id)))
    }

    fn events_path(&self, thread_id: &str) -> PathBuf {
        self.events_dir.join(format!("{}.jsonl", sanitize_id(thread_id)))
    }

    fn approval_decisions_path(&self, thread_id: &str) -> PathBuf {
        self.approval_decisions_dir
            .join(format!("{}.jsonl", sanitize_id(thread_id)))
    }

    fn pending_approval_path(&self, approval_id: &str) -> PathBuf {
        self.pending_approvals_dir
            .join(format!("{}.json", sanitize_id(approval_id)))
    }

    fn load_state(&self) -> Result<RuntimeStoreState> {
        let state: RuntimeStoreState = read_json(&self.state_path)?;
        ensure_supported_schema(state.schema_version, "runtime state")?;
        Ok(state)
    }

    fn save_state(&self, state: &RuntimeStoreState) -> Result<()> {
        write_json_atomic(&self.state_path, state)
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_string_pretty(value)?;
    fs::write(&tmp, data).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn ensure_supported_schema(version: u32, label: &str) -> Result<()> {
    if version > CURRENT_RUNTIME_SCHEMA_VERSION {
        bail!(
            "{} schema v{} is newer than supported v{}",
            label,
            version,
            CURRENT_RUNTIME_SCHEMA_VERSION
        );
    }
    Ok(())
}

fn sanitize_id(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{ApprovalDecisionRecord, RuntimeThreadStore, ThreadRecord, TurnRecord};
    use crate::approval::{MobileApprovalRequest, ReviewDecision};
    use crate::events::AgentEvent;
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use crate::tool_loop::PendingToolCallApproval;
    use crate::tools::file_ops::WriteFileTool;
    use crate::turn::TurnContext;
    use std::fs;

    fn temp_store(name: &str) -> RuntimeThreadStore {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_runtime_store_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        RuntimeThreadStore::open(root).unwrap()
    }

    fn sample_pending() -> PendingToolCallApproval {
        let tool = WriteFileTool;
        let call = ToolCallRequest::new(
            "write_file",
            serde_json::json!({"path":"README.md","content":"x","api_key":"hidden"}),
            ToolCallSource::Manual,
        )
        .with_id("approval-1");
        let approval = MobileApprovalRequest::new(
            "approval-1",
            &tool,
            call.arguments.clone(),
        );
        PendingToolCallApproval { approval, call }
    }

    #[test]
    fn saves_and_loads_thread_and_turn() {
        let store = temp_store("thread_turn");
        let mut thread = ThreadRecord::new("thread-1", "Test", "deepseek-v4-pro", "/workspace");
        let turn = TurnContext::new(10);
        thread.latest_turn_id = Some(turn.id.clone());

        store.save_thread(&thread).unwrap();
        store
            .save_turn(&TurnRecord::from_context(&thread.id, "hello", &turn))
            .unwrap();

        let loaded_thread = store.load_thread("thread-1").unwrap();
        let loaded_turn = store.load_turn(&turn.id).unwrap();

        assert_eq!(loaded_thread.latest_turn_id, Some(turn.id.clone()));
        assert_eq!(loaded_turn.thread_id, "thread-1");
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn appends_and_loads_events() {
        let store = temp_store("events");
        store
            .append_event("thread-1", Some("turn-1".to_string()), AgentEvent::Started)
            .unwrap();
        store
            .append_event("thread-1", Some("turn-1".to_string()), AgentEvent::Finished)
            .unwrap();

        let events = store.load_events("thread-1").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn saves_loads_and_deletes_pending_approval() {
        let store = temp_store("pending_approval");
        let turn = TurnContext::new(10);
        let pending = sample_pending();

        store
            .save_pending_approval("thread-1", &turn.id, &pending)
            .unwrap();
        let loaded = store.load_pending_approval("approval-1").unwrap();
        assert_eq!(loaded.thread_id, "thread-1");
        assert_eq!(loaded.turn_id, turn.id);
        assert_eq!(loaded.pending.call.name, "write_file");

        let by_turn = store.load_pending_approvals_for_turn(&turn.id).unwrap();
        assert_eq!(by_turn.len(), 1);
        store.delete_pending_approval("approval-1").unwrap();
        assert!(store.load_pending_approvals_for_turn(&turn.id).unwrap().is_empty());
        let _ = fs::remove_dir_all(store.root());
    }

    #[test]
    fn appends_and_loads_approval_decisions() {
        let store = temp_store("approval_decisions");
        let pending = sample_pending();
        let record = ApprovalDecisionRecord::new(
            "thread-1",
            "turn-1",
            &pending,
            ReviewDecision::Approved,
            None,
        );
        store.append_approval_decision(record).unwrap();

        let decisions = store.load_approval_decisions("thread-1").unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].seq, 1);
        assert_eq!(decisions[0].approval_id, "approval-1");
        assert_eq!(decisions[0].decision, ReviewDecision::Approved);
        assert_eq!(decisions[0].arguments_preview["api_key"], "<redacted>");

        let by_turn = store.load_approval_decisions_for_turn("turn-1").unwrap();
        assert_eq!(by_turn.len(), 1);
        let _ = fs::remove_dir_all(store.root());
    }
}