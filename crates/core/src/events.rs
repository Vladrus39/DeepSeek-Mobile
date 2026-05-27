//! Agent event types shared by the core and mobile UI.
//!
//! The mobile app should not wait for a single final string forever. The agent
//! core emits timeline events: turn lifecycle, text deltas, reasoning deltas,
//! tool status, approval requests, patch proposals and completion.

use crate::api_client::Message;
use crate::turn::{TokenUsage, TurnStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AgentEvent {
    Started,
    Status(String),
    TurnStarted {
        turn_id: String,
    },
    TurnFinished {
        turn_id: String,
        status: TurnStatus,
        usage: TokenUsage,
        error: Option<String>,
    },
    MessageStarted {
        index: usize,
        role: String,
    },
    TextDelta(String),
    ReasoningDelta(String),
    MessageFinished {
        index: usize,
    },
    ToolCallStarted(ToolCallEvent),
    ToolCallFinished(ToolResultEvent),
    ApprovalRequired(ApprovalRequest),
    TermuxExecutionPending {
        call_id: String,
        request: crate::executor::TermuxExecRequest,
    },
    PatchProposed(PatchProposal),
    SessionUpdated {
        messages: Vec<Message>,
        model: String,
        workspace: String,
    },
    Error(String),
    Finished,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallEvent {
    pub id: String,
    pub name: String,
    pub args: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolResultEvent {
    pub id: String,
    pub name: String,
    pub success: bool,
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Dangerous,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchProposal {
    pub id: String,
    pub file_path: String,
    pub diff: String,
}
