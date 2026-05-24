//! Turn context and tool-call tracking.
//!
//! A turn is one user request plus everything the agent does to answer it:
//! model calls, tool calls, approvals, file edits, command output and final
//! response. This mirrors the original TUI concept while keeping the mobile
//! data model dependency-light.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TURN_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnStatus {
    Queued,
    Running,
    WaitingForApproval,
    WaitingForTermuxResult,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TurnToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
    pub result: Option<String>,
    pub error: Option<String>,
    pub started_at_unix: u64,
    pub finished_at_unix: Option<u64>,
}

impl TurnToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
            result: None,
            error: None,
            started_at_unix: current_unix_time(),
            finished_at_unix: None,
        }
    }

    pub fn set_result(&mut self, result: impl Into<String>) {
        self.result = Some(result.into());
        self.finished_at_unix = Some(current_unix_time());
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.finished_at_unix = Some(current_unix_time());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TurnContext {
    pub id: String,
    pub status: TurnStatus,
    pub step: u32,
    pub max_steps: u32,
    pub tool_calls: Vec<TurnToolCall>,
    pub usage: TokenUsage,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub error: Option<String>,
}

impl TurnContext {
    pub fn new(max_steps: u32) -> Self {
        let now = current_unix_time();
        Self {
            id: new_turn_id(),
            status: TurnStatus::Queued,
            step: 0,
            max_steps,
            tool_calls: Vec::new(),
            usage: TokenUsage::default(),
            created_at_unix: now,
            updated_at_unix: now,
            error: None,
        }
    }

    pub fn start(&mut self) {
        self.status = TurnStatus::Running;
        self.touch();
    }

    pub fn next_step(&mut self) -> bool {
        self.step = self.step.saturating_add(1);
        self.touch();
        self.step <= self.max_steps
    }

    pub fn at_max_steps(&self) -> bool {
        self.step >= self.max_steps
    }

    pub fn record_tool_call(&mut self, call: TurnToolCall) {
        self.tool_calls.push(call);
        self.touch();
    }

    pub fn complete(&mut self) {
        self.status = TurnStatus::Completed;
        self.touch();
    }

    pub fn wait_for_termux(&mut self) {
        self.status = TurnStatus::WaitingForTermuxResult;
        self.touch();
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TurnStatus::Failed;
        self.error = Some(error.into());
        self.touch();
    }

    pub fn cancel(&mut self) {
        self.status = TurnStatus::Cancelled;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at_unix = current_unix_time();
    }
}

fn new_turn_id() -> String {
    let seq = TURN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("turn-{}-{}", current_unix_time(), seq)
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{TurnContext, TurnStatus, TurnToolCall};
    use serde_json::json;

    #[test]
    fn creates_and_completes_turn() {
        let mut turn = TurnContext::new(3);
        assert_eq!(turn.status, TurnStatus::Queued);
        turn.start();
        assert_eq!(turn.status, TurnStatus::Running);
        turn.complete();
        assert_eq!(turn.status, TurnStatus::Completed);
    }

    #[test]
    fn enforces_step_limit() {
        let mut turn = TurnContext::new(1);
        assert!(turn.next_step());
        assert!(!turn.next_step());
        assert!(turn.at_max_steps());
    }

    #[test]
    fn records_tool_calls() {
        let mut turn = TurnContext::new(5);
        let mut call = TurnToolCall::new("tool-1", "read_file", json!({"path": "README.md"}));
        call.set_result("ok");
        turn.record_tool_call(call);
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "read_file");
    }
}